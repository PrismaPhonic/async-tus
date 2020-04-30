use std::core::task::Context;

use tokio::io::AsyncRead;
use crossbeam::channel::Sender;

use crate::Error;

const CHUNK_SIZE: usize = 1024 * 1024 * 4;

type Chunk = [u8; CHUNK_SIZE];

pub struct Processor<T> 
    where T: AsyncRead
{
    reader: T,
    sender: Sender<(Chunk, u32)>,
}

impl<T> Processor<T>
    where T: AsyncRead
{
    pub fn new(reader: T, sender: Sender<(Chunk, u32)>) -> Processor<T> {
        Processor {
            reader,
            sender,
        }
    }

    pub async fn process(&mut self, ctx: Context) -> Result<usize, Error> {
        let mut chunk: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];
        let bytes_read = self.reader.poll_read(ctx)
            .await
            .map_err(|e| Error:IoError(e))?;

        self.sender
        Ok(0)
    }
}
