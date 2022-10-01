use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;

use crate::blorb_chunk_types::BlorbChunkType;
use crate::ulx_reader::UlxReader;
use crate::FileReadError::{InvalidLength, UnexpectedStartingIdentifier};
use crate::{read_be_u32, FileReadError};

struct FileIndex<'a>(HashMap<BlorbChunkType, HashMap<i32, Chunk<'a>>>);

pub struct BlorbReader<'a> {
    file_index: FileIndex<'a>,
    // optional_fields_used: Vec<BlorbChunkType>,
}

impl Display for BlorbReader<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str("BlorbReader{ file_index: ")?;
        for (k, v) in &self.file_index.0 {
            f.write_fmt(format_args!("{:?}{{", k))?;
            for (k2, v2) in v {
                f.write_fmt(format_args!("ID: {}, {}", k2, v2))?;
            }
            f.write_str("}, ")?;
        }
        f.write_str("}")?;
        Ok(())
    }
}

pub enum ChunkData<'a> {
    Executable(&'a [u8]),
    PNG(PngDecoder<&'a [u8]>),
    JPG(JpegDecoder<&'a [u8]>),
}

pub struct Chunk<'a> {
    pub chunk_type: BlorbChunkType,
    pub data: ChunkData<'a>,
}

impl<'a> TryFrom<&'a [u8]> for Chunk<'a> {
    type Error = FileReadError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() < 8 {
            return Err(InvalidLength(value.len(), 8));
        }
        let chunk_type = read_be_u32(&value[..4]).try_into()?;
        let len = read_be_u32(&value[4..8]);
        let data = match chunk_type {
            BlorbChunkType::PICTURE_PNG => ChunkData::PNG(PngDecoder::new(&value[8..(len as usize)]).unwrap()),
            BlorbChunkType::PICTURE_JPEG => ChunkData::JPG(JpegDecoder::new(&value[8..(len as usize)]).unwrap()),
            BlorbChunkType::EXEC_GLUL => ChunkData::Executable(&value[8..(len as usize)]),
            _ => panic!()
        };
        Ok(Chunk { chunk_type, data })
    }
}

impl<'a> Display for Chunk<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("Chunk type: {:?}", self.chunk_type,))?;
        Ok(())
    }
}

impl<'a> TryFrom<&'a [u8]> for FileIndex<'a> {
    type Error = FileReadError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        const INDEX_HEADER_SIZE: usize = 12;
        const CHUNK_HEADER_SIZE: usize = 12;
        if read_be_u32(&value[..4]) != BlorbChunkType::RESOURCE_INDEX as u32 {
            return Err(UnexpectedStartingIdentifier(
                BlorbChunkType::RESOURCE_INDEX,
            ));
        }
        // let index_len = read_be_u32(&v[4..8]);
        let num_in_index = read_be_u32(&value[8..12]);

        let mut ret = HashMap::new();
        ret.insert(BlorbChunkType::PICTURE, HashMap::new());
        ret.insert(BlorbChunkType::SOUND, HashMap::new());
        ret.insert(BlorbChunkType::DATA, HashMap::new());
        ret.insert(BlorbChunkType::EXECUTABLE, HashMap::new());

        for i in 0..num_in_index {
            let offset = INDEX_HEADER_SIZE + (i as usize * CHUNK_HEADER_SIZE);
            let key = read_be_u32(&value[offset..(offset + 4)]).try_into()?;
            let id = i32::from_be_bytes(value[(offset + 4)..(offset + 8)].try_into().unwrap());
            let address = read_be_u32(&value[(offset + 8)..(offset + 12)]);

            // The address is from the start of the blorb, which is 12 bytes earlier than the
            // start of the index (the value here) (FORM, len, IFRS).
            ret.entry(key)
                .or_insert_with(HashMap::new)
                .insert(id, value[address as usize - 12..].try_into().unwrap());
        }

        Ok(FileIndex(ret))
    }
}

impl<'a> TryFrom<&'a [u8]> for BlorbReader<'a> {
    type Error = FileReadError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if read_be_u32(&value[..4]) != BlorbChunkType::FORM as u32 {
            return Err(UnexpectedStartingIdentifier(BlorbChunkType::FORM));
        }
        if read_be_u32(&value[4..8]) != (value.len() - 8) as u32 {
            return Err(InvalidLength(
                value.len() - 8,
                read_be_u32(&value[4..8]) as usize,
            ));
        }
        if read_be_u32(&value[8..12]) != BlorbChunkType::IFRS as u32 {
            return Err(UnexpectedStartingIdentifier(BlorbChunkType::IFRS));
        }

        let file_index = value[12..].try_into()?;

        Ok(BlorbReader { file_index })
    }
}

impl<'a> BlorbReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<BlorbReader<'a>, FileReadError> {
        bytes.try_into()
    }

    pub fn get_exec(&'a self, id: i32) -> Option<UlxReader> {
        let c = self
            .file_index
            .0
            .get(&BlorbChunkType::EXECUTABLE)?
            .get(&id)?;
        match c.data {
            ChunkData::Executable(data) => data.try_into().ok(),
            _ => None,
        }
    }

    pub fn get_image(&'a self, id: i32) -> Option<&'a ChunkData<'a>> {
        let c = self.get(BlorbChunkType::PICTURE, id);
        if let Some(ChunkData::Executable(_)) = c {
            return None;
        }
        c
    }

    pub fn get(&'a self, chunk_type: BlorbChunkType, id: i32) -> Option<&'a ChunkData<'a>> {
        Some(&self
            .file_index
            .0
            .get(&chunk_type)?
            .get(&id)?
            .data)
    }
}
