use tokio::io::{AsyncRead, AsyncReadExt};

use crate::Error;
use std::task::Context;
use tokio::sync::mpsc::Sender;

const CHUNK_SIZE: usize = 1024 * 1024 * 4;

pub(crate) type Chunk = Vec<u8>;

pub struct Processor<T> 
    where T: AsyncReadExt + Unpin + Send
{
    reader: T,
    sender: Sender<(Chunk, usize)>,
}

impl<T> Processor<T>
    where T: AsyncReadExt + Unpin + Send
{
    pub fn new(reader: T, sender: Sender<(Chunk, usize)>) -> Processor<T> {
        Processor {
            reader,
            sender,
        }
    }

    pub async fn process(&mut self) -> Result<(), Error> {
        loop {
            let mut chunk = vec![0; CHUNK_SIZE];
            let bytes_read = self.reader.read(&mut chunk)
                .await
                .map_err(|e| Error::IoError(e))?;
            if bytes_read == 0 {
                break
            }
            chunk.truncate(bytes_read);

            // TODO: Potentially limit channel size so we force pause here to keep
            // channel from getting too filled up.
            self.sender.send((chunk, bytes_read))
                .await
                .map_err(|e| Error::ChannelError(e))?;
        }
        Ok(())
    }
}
