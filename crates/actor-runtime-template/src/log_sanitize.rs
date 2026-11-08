//! Bounded, injection-safe rendering of actor payloads for logging.
//!
//! Actor work payloads are untrusted: they can contain control or Bidi
//! characters that forge log records, and they can be arbitrarily long. These
//! helpers escape the dangerous characters and cap the rendered length.

/// Upper bound (in escaped characters) for a work payload rendered into a log line.
pub const MAX_LOGGED_LEN: usize = 256;

/// Escape control/Bidi/format characters and bound the length of a payload for logging.
///
/// Runs as a single bounded pass: a source character is emitted only when its whole escape
/// sequence fits within [`MAX_LOGGED_LEN`], so the payload is never materialised in full and a
/// truncation suffix can never split an escape sequence mid-way.
///
/// Returns the escaped string, or the escaped prefix followed by `... [truncated]`.
pub fn sanitize_log_payload(work: &str) -> String {
    let i = scan_printable_prefix(work.as_bytes());
    if i == work.len() {
        return truncate_plain(work);
    }
    truncate_or_escape_remainder(work, i)
}

/// Return `work` unchanged when it fits, else its prefix plus a truncation suffix.
pub fn truncate_plain(work: &str) -> String {
    if work.len() <= MAX_LOGGED_LEN {
        return work.to_string();
    }
    let mut out = String::with_capacity(MAX_LOGGED_LEN + "... [truncated]".len());
    out.push_str(&work[..MAX_LOGGED_LEN]);
    out.push_str("... [truncated]");
    out
}

/// Return the byte index of the first character needing escaping or truncation.
///
/// Uses a SWAR (SIMD Within A Register) fast path that checks eight bytes at a
/// time for printable ASCII (`0x20..=0x7E`) excluding `\\`, `"`, and `'`, then
/// finishes a scalar loop over the remaining bytes.
pub fn scan_printable_prefix(bytes: &[u8]) -> usize {
    let mut i = 0;

    // Fast path: SWAR check 8-byte chunks simultaneously for printable ASCII
    // values (0x20-0x7E) excluding '\\', '"', and '\''. Uses from_le_bytes for
    // deterministic cross-endian behavior.
    while i + 8 <= bytes.len() {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[i..i + 8]);
        let chunk = u64::from_le_bytes(arr);

        let has_low = (chunk.wrapping_sub(0x2020_2020_2020_2020) & !chunk) & 0x8080_8080_8080_8080;
        let has_high = chunk & 0x8080_8080_8080_8080;
        let y = chunk ^ 0x7F7F_7F7F_7F7F_7F7F;
        let zero_check = (y.wrapping_sub(0x0101_0101_0101_0101) & !y) & 0x8080_8080_8080_8080;

        let contains_byte = |b: u8| {
            let mask = u64::from_le_bytes([b; 8]);
            let x = chunk ^ mask;
            (x.wrapping_sub(0x0101_0101_0101_0101) & !x) & 0x8080_8080_8080_8080
        };

        if (has_low
            | has_high
            | zero_check
            | contains_byte(b'\\')
            | contains_byte(b'"')
            | contains_byte(b'\''))
            != 0
        {
            break;
        }
        i += 8;
    }

    // Scalar fallback loop for remaining bytes.
    while i < bytes.len() {
        let b = bytes[i];
        if !(0x20..=0x7E).contains(&b) || b == b'\\' || b == b'"' || b == b'\'' {
            break;
        }
        i += 1;
    }

    i
}

/// Escape and truncate `work` starting from the first unsafe byte `i`.
pub fn truncate_or_escape_remainder(work: &str, i: usize) -> String {
    if i >= MAX_LOGGED_LEN {
        return truncate_plain(work);
    }

    // Slow path: some characters need escaping before MAX_LOGGED_LEN.
    // Batch contiguous safe character segments with `push_str` to minimize allocation and copying overhead,
    // while caching `escape_debug()` for unsafe characters in a stack buffer.
    let mut out = String::with_capacity(work.len().min(MAX_LOGGED_LEN) + 16);
    out.push_str(&work[..i]);
    let mut used = i;

    let mut safe_start: Option<usize> = None;
    let mut safe_char_count = 0usize;

    for (idx, ch) in work[i..].char_indices() {
        let mut esc_buf = ['\0'; 12];
        let mut esc_len = 0;
        for esc_ch in ch.escape_debug() {
            esc_buf[esc_len] = esc_ch;
            esc_len += 1;
        }

        let is_safe = esc_len == 1 && esc_buf[0] == ch;

        if is_safe {
            if safe_start.is_none() {
                safe_start = Some(idx);
            }
            safe_char_count += 1;
        } else {
            if let Some(start) = safe_start.take() {
                let safe_slice = &work[i + start..i + idx];
                if used + safe_char_count > MAX_LOGGED_LEN {
                    let take_len = MAX_LOGGED_LEN - used;
                    for c in safe_slice.chars().take(take_len) {
                        out.push(c);
                    }
                    out.push_str("... [truncated]");
                    return out;
                }
                out.push_str(safe_slice);
                used += safe_char_count;
                safe_char_count = 0;
            }

            if used + esc_len > MAX_LOGGED_LEN {
                out.push_str("... [truncated]");
                return out;
            }

            out.extend(&esc_buf[..esc_len]);
            used += esc_len;
        }
    }

    if let Some(start) = safe_start {
        let safe_slice = &work[i + start..];
        if used + safe_char_count > MAX_LOGGED_LEN {
            let take_len = MAX_LOGGED_LEN - used;
            for c in safe_slice.chars().take(take_len) {
                out.push(c);
            }
            out.push_str("... [truncated]");
            return out;
        }
        out.push_str(safe_slice);
    }

    out
}
