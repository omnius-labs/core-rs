pub fn increment_bytes(bytes: &mut [u8]) {
    for b in bytes {
        if *b == 0xFF {
            *b = 0;
        } else {
            *b += 1;
            break;
        }
    }
}
