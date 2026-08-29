// omnius-lint:debt(free-fn) nonce を表す型が欠けており、Aes256GcmEncoder と Aes256GcmDecoder が共有する所有者の新設を暗号処理の変更と分ける
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
