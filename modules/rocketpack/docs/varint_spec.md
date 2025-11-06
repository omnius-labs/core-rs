# RocketPack Varint Specification

## Overview
RocketPack varints encode unsigned integers using a 1-byte header followed by an optional
little-endian payload. Values between `0x00` and `0x7F` (inclusive) are represented directly
by the header byte. Larger values use specific header markers (`0x80`–`0x84`) to signal how
many payload bytes follow. The design keeps small values compact while falling back to
fixed-width payloads for larger values.

## Encoding Format

| Header | Payload Length | Value Range                                      |
| ------ | -------------- | ------------------------------------------------ |
| 0x00–0x7F | 0 bytes        | Encoded value equals the header byte.            |
| 0x80   | 1 byte         | Next byte contains an unsigned 8-bit value.       |
| 0x81   | 2 bytes        | Next two bytes contain an unsigned 16-bit value.  |
| 0x82   | 4 bytes        | Next four bytes contain an unsigned 32-bit value. |
| 0x83   | 8 bytes        | Next eight bytes contain an unsigned 64-bit value.|
| 0x84   | 8 bytes        | Next eight bytes contain an unsigned 128-bit value.|

All payloads are serialized in little-endian order.

## Signed Integer Encoding
Signed integers are mapped to unsigned integers using ZigZag encoding prior to varint
serialization. ZigZag encoding converts an `n`-bit signed integer to an `n`-bit unsigned
integer by left-shifting the value by one bit and XOR-ing with the sign bit cast to
unsigned form. Decoding reverses this transformation after the varint is read. This allows
small negative values to retain a compact 1-byte representation.
