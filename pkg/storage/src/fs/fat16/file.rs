//! File
//!
//! reference: <https://wiki.osdev.org/FAT#Directories_on_FAT12.2F16.2F32>

use super::*;

#[derive(Debug, Clone)]
pub struct File {
    /// The current offset in the file
    offset: usize,
    /// The current cluster of this file
    current_cluster: Cluster,
    /// DirEntry of this file
    entry: DirEntry,
    /// The file system handle that contains this file
    handle: Fat16Handle,
}

impl File {
    pub fn new(handle: Fat16Handle, entry: DirEntry) -> Self {
        Self {
            offset: 0,
            current_cluster: entry.cluster,
            entry,
            handle,
        }
    }

    pub fn length(&self) -> usize {
        self.entry.size as usize
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // FIXME: read file content from disk
        //      CAUTION: file length / buffer size / offset
        //
        //      - `self.offset` is the current offset in the file in bytes
        //      - use `self.handle` to read the blocks
        //      - use `self.entry` to get the file's cluster
        //      - use `self.handle.cluster_to_sector` to convert cluster to sector
        //      - update `self.offset` after reading
        //      - update `self.cluster` with FAT if necessary
        // Check buffer length and file length
        if buf.is_empty() {
            return Ok(0);
        }
        let file_len = self.length();
        if self.offset >= file_len {
            return Ok(0);
        }

        let to_read = core::cmp::min(buf.len(), file_len - self.offset);
        let mut read_bytes = 0usize;

        // Prepare the variables for reading
        let blk_size = Block512::size();
        let sec_per_clus = self.handle.bpb.sectors_per_cluster() as usize;
        let clus_size = sec_per_clus * blk_size;

        let mut cluster = self.current_cluster;
        let mut block = Block::default();

        // Fill the buffer until we reach the end of the file or the buffer is full
        while read_bytes < to_read {
            let in_clus_off = self.offset % clus_size;
            let remain_in_clus = clus_size - in_clus_off;

            let mut sector = self.handle.cluster_to_sector(&cluster) + in_clus_off / blk_size;
            let mut in_sector_off = in_clus_off % blk_size;

            let mut left = core::cmp::min(remain_in_clus, to_read - read_bytes);

            while left > 0 {
                let copy_len = core::cmp::min(blk_size - in_sector_off, left);

                // Read the block from disk
                self.handle.inner.read_block(sector, &mut block)?;

                // Copy data from the block to the buffer
                buf[read_bytes..read_bytes + copy_len]
                    .copy_from_slice(&block[in_sector_off..in_sector_off + copy_len]);

                // Update the read bytes and offset
                read_bytes += copy_len;
                self.offset += copy_len;
                left -= copy_len;

                // Update the sector and in-sector offset
                sector += 1;
                in_sector_off = 0;
            }

            if read_bytes < to_read {
                cluster = match self.handle.next_cluster(cluster)? {
                    Cluster::END_OF_FILE => break,
                    c => {
                        self.current_cluster = c;
                        c
                    }
                };
            }
        }

        Ok(read_bytes)
    }
}

// NOTE: `Seek` trait is not required for this lab
impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> Result<usize> {
        unimplemented!()
    }
}

// NOTE: `Write` trait is not required for this lab
impl Write for File {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> Result<()> {
        unimplemented!()
    }
}
