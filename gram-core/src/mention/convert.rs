/// 把 UTF-16 的 [offset, offset+len) 区间映射成 UTF-8 字节区间。
///
/// 返回 `Some((byte_start, byte_end))`，如果越界则返回 `None`。
pub fn utf16_range_to_utf8(s: &str, offset: usize, len: usize) -> Option<(usize, usize)> {
    let utf16_to_byte_idx = |idx: usize| -> Option<usize> {
        let mut utf16_cnt = 0;
        for (byte_idx, ch) in s.char_indices() {
            if utf16_cnt == idx {
                return Some(byte_idx);
            }
            utf16_cnt += ch.len_utf16();
        }
        // 尾部也允许（例如空区间放在末尾）
        if utf16_cnt == idx {
            return Some(s.len());
        }
        None
    };

    let start = utf16_to_byte_idx(offset)?;
    let end = utf16_to_byte_idx(offset + len)?;
    Some((start, end))
}
