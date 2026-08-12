use super::manager::CacheMetricsSnapshot;
use serde::{Serialize, de::DeserializeOwned};
use std::io;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;

const IO_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Default)]
struct CacheMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    writes: AtomicU64,
    invalidations: AtomicU64,
    errors: AtomicU64,
}

struct RedisConnection {
    reader: BufReader<TcpStream>,
}

enum RedisResponse {
    Simple(String),
    Integer(i64),
    Bulk(Option<Vec<u8>>),
}

pub struct RedisPool {
    address: String,
    namespace: String,
    connections: Vec<Mutex<Option<RedisConnection>>>,
    next: AtomicUsize,
    metrics: CacheMetrics,
}

impl RedisPool {
    pub fn new(url: &str, pool_size: usize, namespace: &str) -> Self {
        let address = url
            .strip_prefix("redis://")
            .unwrap_or(url)
            .trim_end_matches('/')
            .to_string();
        let size = pool_size.clamp(1, 128);

        Self {
            address,
            namespace: namespace.to_string(),
            connections: (0..size).map(|_| Mutex::new(None)).collect(),
            next: AtomicUsize::new(0),
            metrics: CacheMetrics::default(),
        }
    }

    pub fn metrics_snapshot(&self) -> CacheMetricsSnapshot {
        let hits = self.metrics.hits.load(Ordering::Relaxed);
        let misses = self.metrics.misses.load(Ordering::Relaxed);
        let requests = hits + misses;

        CacheMetricsSnapshot {
            hits,
            misses,
            writes: self.metrics.writes.load(Ordering::Relaxed),
            invalidations: self.metrics.invalidations.load(Ordering::Relaxed),
            errors: self.metrics.errors.load(Ordering::Relaxed),
            hit_rate_percent: if requests == 0 {
                0.0
            } else {
                hits as f64 * 100.0 / requests as f64
            },
        }
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{key}", self.namespace)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> io::Result<Option<T>> {
        let full_key = self.key(key);
        match self.command(&[b"GET", full_key.as_bytes()]).await {
            Ok(RedisResponse::Bulk(Some(bytes))) => match serde_json::from_slice(&bytes) {
                Ok(value) => {
                    self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                    Ok(Some(value))
                }
                Err(error) => {
                    self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                    Err(io::Error::new(io::ErrorKind::InvalidData, error))
                }
            },
            Ok(RedisResponse::Bulk(None)) => {
                self.metrics.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
            Ok(_) => {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unexpected Redis GET response",
                ))
            }
            Err(error) => {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    }

    pub async fn set_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: u64,
    ) -> io::Result<()> {
        let full_key = self.key(key);
        let ttl = ttl_seconds.to_string();
        let payload = serde_json::to_vec(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        match self
            .command(&[
                b"SETEX",
                full_key.as_bytes(),
                ttl.as_bytes(),
                payload.as_slice(),
            ])
            .await
        {
            Ok(RedisResponse::Simple(value)) if value == "OK" => {
                self.metrics.writes.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected Redis SETEX response",
            )),
            Err(error) => {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    }

    pub async fn version(&self, family: &str) -> io::Result<i64> {
        let key = self.key(&format!("version:{family}"));
        match self.command(&[b"GET", key.as_bytes()]).await? {
            RedisResponse::Bulk(Some(bytes)) => String::from_utf8(bytes)
                .ok()
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad version")),
            RedisResponse::Bulk(None) => Ok(0),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected Redis version response",
            )),
        }
    }

    pub async fn bump_version(&self, family: &str) -> io::Result<i64> {
        let key = self.key(&format!("version:{family}"));
        match self.command(&[b"INCR", key.as_bytes()]).await {
            Ok(RedisResponse::Integer(value)) => {
                self.metrics.invalidations.fetch_add(1, Ordering::Relaxed);
                Ok(value)
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected Redis INCR response",
            )),
            Err(error) => {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    }

    async fn command(&self, parts: &[&[u8]]) -> io::Result<RedisResponse> {
        let index = self.next.fetch_add(1, Ordering::Relaxed) % self.connections.len();
        let mut slot = self.connections[index].lock().await;

        for attempt in 0..2 {
            let mut connection = if let Some(connection) = slot.take() {
                connection
            } else {
                let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(&self.address))
                    .await
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::TimedOut, "Redis connect timeout")
                    })??;
                let _ = stream.set_nodelay(true);
                RedisConnection {
                    reader: BufReader::new(stream),
                }
            };

            // Keep the connection outside the pool slot while I/O is in flight.
            // If this future is cancelled, the connection is dropped instead of
            // returning to the pool with an unread response.
            let result = Self::send_and_receive(&mut connection, parts).await;
            match result {
                Ok(response) => {
                    *slot = Some(connection);
                    return Ok(response);
                }
                Err(error) if attempt == 0 => {
                    if error.kind() == io::ErrorKind::InvalidData {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Redis command failed",
        ))
    }

    async fn send_and_receive(
        connection: &mut RedisConnection,
        parts: &[&[u8]],
    ) -> io::Result<RedisResponse> {
        let command = encode_command(parts);
        timeout(IO_TIMEOUT, connection.reader.get_mut().write_all(&command))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Redis write timeout"))??;
        timeout(IO_TIMEOUT, connection.reader.get_mut().flush())
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Redis flush timeout"))??;
        timeout(IO_TIMEOUT, read_response(&mut connection.reader))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Redis read timeout"))?
    }
}

fn encode_command(parts: &[&[u8]]) -> Vec<u8> {
    let mut output = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        output.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        output.extend_from_slice(part);
        output.extend_from_slice(b"\r\n");
    }
    output
}

async fn read_response(reader: &mut BufReader<TcpStream>) -> io::Result<RedisResponse> {
    let mut prefix = [0_u8; 1];
    reader.read_exact(&mut prefix).await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let value = line.trim_end_matches(['\r', '\n']);

    match prefix[0] {
        b'+' => Ok(RedisResponse::Simple(value.to_string())),
        b':' => value
            .parse()
            .map(RedisResponse::Integer)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        b'$' => {
            let length: i64 = value
                .parse()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            if length < 0 {
                return Ok(RedisResponse::Bulk(None));
            }

            let mut bytes = vec![0_u8; length as usize];
            reader.read_exact(&mut bytes).await?;
            let mut terminator = [0_u8; 2];
            reader.read_exact(&mut terminator).await?;
            if terminator != *b"\r\n" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid Redis bulk terminator",
                ));
            }
            Ok(RedisResponse::Bulk(Some(bytes)))
        }
        b'-' => Err(io::Error::other(format!("Redis error: {value}"))),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported Redis response",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_binary_safe_resp_commands() {
        assert_eq!(
            encode_command(&[b"SETEX", b"quest:key", b"60", b"{\"ok\":true}"]),
            b"*4\r\n$5\r\nSETEX\r\n$9\r\nquest:key\r\n$2\r\n60\r\n$11\r\n{\"ok\":true}\r\n"
        );
    }
}
