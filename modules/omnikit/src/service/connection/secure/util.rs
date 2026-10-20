// omnius-lint:debt(free-fn) There is no type that represents a nonce; introducing an owner shared by Aes256GcmEncoder and Aes256GcmDecoder is kept separate from changes to the cryptographic processing
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
