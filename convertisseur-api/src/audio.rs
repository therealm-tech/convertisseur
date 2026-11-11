//! Uploaded recordings: what a file is, judged from its first bytes, and
//! how long it lasts when the file says so cheaply. Files are stored as
//! uploaded, never transcoded.

/// The largest file accepted, 10 MB.
pub const MAX_BYTES: usize = 10 * 1024 * 1024;

/// The longest length a client may state, 10 minutes.
pub const MAX_CLAIMED_DURATION_MS: u32 = 10 * 60 * 1000;

/// A container every current browser plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Mp3,
    /// MP4 audio, AAC as Safari records it, or an `.m4a`.
    Mp4,
    /// Opus or Vorbis in Ogg, as Firefox records it.
    Ogg,
    /// Opus in `WebM`, as Chrome records it.
    WebM,
    Wav,
}

impl Format {
    /// What the bytes are, whatever the client called them.
    #[must_use]
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [b'I', b'D', b'3', ..] => Some(Self::Mp3),
            // An MPEG audio frame header: 11 sync bits, then a layer other
            // than the 00 that ADTS AAC uses.
            [0xFF, second, ..] if second & 0xE0 == 0xE0 && second & 0x06 != 0 => Some(Self::Mp3),
            [_, _, _, _, b'f', b't', b'y', b'p', ..] => Some(Self::Mp4),
            [b'O', b'g', b'g', b'S', ..] => Some(Self::Ogg),
            [0x1A, 0x45, 0xDF, 0xA3, ..] => Some(Self::WebM),
            [
                b'R',
                b'I',
                b'F',
                b'F',
                _,
                _,
                _,
                _,
                b'W',
                b'A',
                b'V',
                b'E',
                ..,
            ] => Some(Self::Wav),
            _ => None,
        }
    }

    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Mp4 => "audio/mp4",
            Self::Ogg => "audio/ogg",
            Self::WebM => "audio/webm",
            Self::Wav => "audio/wav",
        }
    }

    /// The length in milliseconds, read from the WAV header or the last Ogg
    /// page; `None` for the other formats, whose length takes parsing the
    /// whole stream, and for a file that does not say.
    #[must_use]
    pub fn duration_ms(self, bytes: &[u8]) -> Option<u32> {
        let ms = match self {
            Self::Wav => wav_duration_ms(bytes)?,
            Self::Ogg => ogg_duration_ms(bytes)?,
            Self::Mp3 | Self::Mp4 | Self::WebM => return None,
        };
        u32::try_from(ms).ok()
    }
}

fn u16_le(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_le(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_le(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// The `data` chunk's size over the `fmt ` chunk's byte rate.
fn wav_duration_ms(bytes: &[u8]) -> Option<u64> {
    let mut byte_rate = None;
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u64::from(u32_le(bytes, at + 4)?);
        match id {
            b"fmt " => byte_rate = Some(u64::from(u32_le(bytes, at + 16)?)),
            b"data" => {
                let rate = byte_rate.filter(|&rate| rate > 0)?;
                return Some(size * 1000 / rate);
            }
            _ => {}
        }
        // Chunks are padded to an even size.
        at = at.checked_add(usize::try_from(size + size % 2).ok()? + 8)?;
    }
    None
}

/// The granule position of the last page, in samples at the stream's rate:
/// 48 kHz minus the pre-skip for Opus, the header's rate for Vorbis.
fn ogg_duration_ms(bytes: &[u8]) -> Option<u64> {
    let last_page = bytes.windows(4).rposition(|w| w == b"OggS")?;
    let granule = u64_le(bytes, last_page + 6)?;
    // The first packet starts after the first page's 27-byte header and
    // its segment table.
    let segments = usize::from(*bytes.get(26)?);
    let packet = bytes.get(27 + segments..)?;
    if packet.starts_with(b"OpusHead") {
        let pre_skip = u64::from(u16_le(packet, 10)?);
        Some(granule.saturating_sub(pre_skip) * 1000 / 48_000)
    } else if packet.get(1..7) == Some(b"vorbis") {
        let rate = u64::from(u32_le(packet, 12)?);
        (rate > 0).then(|| granule * 1000 / rate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Format;

    fn wav(seconds: u32) -> Vec<u8> {
        let rate = 8_000_u32;
        let data = rate * seconds;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data.to_le_bytes());
        bytes.resize(bytes.len() + data as usize, 0x80);
        bytes
    }

    fn ogg_page(granule: u64, packet: &[u8]) -> Vec<u8> {
        let mut page = b"OggS\0\0".to_vec();
        page.extend_from_slice(&granule.to_le_bytes());
        page.extend_from_slice(&[0; 12]);
        page.push(1);
        page.push(u8::try_from(packet.len()).unwrap());
        page.extend_from_slice(packet);
        page
    }

    fn opus(granule: u64) -> Vec<u8> {
        let mut head = b"OpusHead\x01\x01".to_vec();
        head.extend_from_slice(&312_u16.to_le_bytes());
        head.extend_from_slice(&48_000_u32.to_le_bytes());
        let mut bytes = ogg_page(0, &head);
        bytes.extend(ogg_page(granule, b"audio"));
        bytes
    }

    #[test]
    fn recognises_what_browsers_record_and_play() {
        let cases: [(&[u8], Format); 7] = [
            (b"ID3\x04\0\0\0\0\0\0", Format::Mp3),
            (&[0xFF, 0xFB, 0x90, 0x64], Format::Mp3),
            (b"\0\0\0\x20ftypM4A \0\0\0\0", Format::Mp4),
            (b"\0\0\0\x18ftypiso5\0\0\0\x01", Format::Mp4),
            (b"OggS\0\x02", Format::Ogg),
            (&[0x1A, 0x45, 0xDF, 0xA3, 0x9F, 0x42], Format::WebM),
            (b"RIFF\x24\0\0\0WAVEfmt ", Format::Wav),
        ];
        for (bytes, format) in cases {
            assert_eq!(Format::sniff(bytes), Some(format), "{bytes:x?}");
        }
    }

    #[test]
    fn refuses_anything_else() {
        let cases: [&[u8]; 7] = [
            b"",
            b"ID",
            b"<html><body>",
            b"%PDF-1.7",
            b"\x89PNG\r\n\x1a\n",
            // ADTS AAC: the sync bits of MPEG audio, with layer 00.
            &[0xFF, 0xF1, 0x50, 0x80],
            b"RIFF\x24\0\0\0AVI LIST",
        ];
        for bytes in cases {
            assert_eq!(Format::sniff(bytes), None, "{bytes:x?}");
        }
    }

    #[test]
    fn reads_the_length_of_wav_and_ogg_opus() {
        assert_eq!(Format::Wav.duration_ms(&wav(3)), Some(3_000));
        assert_eq!(Format::Ogg.duration_ms(&opus(96_312)), Some(2_000));
        assert_eq!(Format::Wav.duration_ms(b"RIFF\0\0\0\0WAVE"), None);
        assert_eq!(Format::Mp3.duration_ms(b"ID3\x04"), None);
    }
}
