#!/usr/bin/env python3
"""Independent V2 fixtures; all private keys below are public test material."""

import argparse
import hashlib
import hmac
import json
import struct
from pathlib import Path

from cryptography.exceptions import InvalidSignature, InvalidTag
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ed25519, x25519
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.hkdf import HKDF, HKDFExpand


def label(text):
    return ("omnius.secure.v2/" + text).encode("ascii") + b"\0"


def digest(value):
    return hashlib.sha3_256(value).digest()


def mac(key, value):
    return hmac.digest(key, value, "sha3_256")


def lp(value):
    return struct.pack("<I", len(value)) + value


def expand(key, info, length):
    # RFC 5869 recurrence, cross-checked against the OpenSSL-backed API.
    previous, output = b"", b""
    for index in range(1, (length + 31) // 32 + 1):
        previous = mac(key, previous + info + bytes([index]))
        output += previous
    result = output[:length]
    assert result == HKDFExpand(hashes.SHA3_256(), length, info).derive(key)
    return result


def cbor_head(major, value):
    if value < 24:
        return bytes([(major << 5) | value])
    for size, tag in [(1, 24), (2, 25), (4, 26), (8, 27)]:
        if value < 1 << (size * 8):
            return bytes([(major << 5) | tag]) + value.to_bytes(size, "big")
    raise ValueError("CBOR integer overflow")


def cbor(value):
    if isinstance(value, int):
        return cbor_head(0, value)
    if isinstance(value, bytes):
        return cbor_head(2, len(value)) + value
    if isinstance(value, str):
        encoded = value.encode("utf-8")
        return cbor_head(3, len(encoded)) + encoded
    if isinstance(value, dict):
        return cbor_head(5, len(value)) + b"".join(cbor(k) + cbor(v) for k, v in value.items())
    raise TypeError(type(value))


def frame(value):
    encoded = cbor(value)
    return lp(encoded)


def canonical_profile(role, mode, context, nonce, ephemeral, name, public):
    return struct.pack("<I", 2) + bytes([role, mode]) + lp(context) + nonce + ephemeral + lp(name.encode()) + lp(public)


def canonical_auth(cert):
    if cert is None:
        return b"\0"
    return b"\1" + struct.pack("<I", 2) + lp(cert[2].encode()) + lp(cert[3]) + lp(cert[4])


def epoch(secret, transcript, direction, generation):
    suffix = transcript + bytes([direction]) + struct.pack("<Q", generation)
    return expand(secret, label("key") + suffix, 32), expand(secret, label("iv") + suffix, 12)


def next_secret(secret, transcript, direction, generation):
    return expand(secret, label("update") + transcript + bytes([direction]) + struct.pack("<Q", generation), 32)


def record(key, iv, transcript, direction, generation, sequence, kind, plaintext):
    header = struct.pack("<BQQI", kind, generation, sequence, len(plaintext) + 16)
    nonce = bytes(a ^ b for a, b in zip(iv, b"\0" * 4 + sequence.to_bytes(8, "big")))
    aad = label("record") + transcript + bytes([direction]) + header
    assert len(header) == 21 and len(aad) == 78
    ciphertext = AESGCM(key).encrypt(nonce, plaintext, aad)
    assert AESGCM(key).decrypt(nonce, ciphertext, aad) == plaintext
    return {
        "direction": direction, "generation": generation, "sequence": sequence, "kind": kind,
        "plaintext_hex": plaintext.hex(), "header_hex": header.hex(), "nonce_hex": nonce.hex(),
        "aad_hex": aad.hex(), "ciphertext_hex": ciphertext.hex(), "wire_hex": (header + ciphertext).hex(),
    }


def make_case(signed):
    context = b"axus-session-v2"
    dh_seeds = [bytes(range(32)), bytes(range(32, 64))]
    sign_seeds = [bytes(range(64, 96)), bytes(range(96, 128))]
    nonces = [bytes(range(160, 192)), bytes(range(192, 224))]
    dh = [x25519.X25519PrivateKey.from_private_bytes(seed) for seed in dh_seeds]
    ephemeral = [key.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw) for key in dh]
    signing = [ed25519.Ed25519PrivateKey.from_private_bytes(seed) for seed in sign_seeds]
    public = [key.public_key().public_bytes(serialization.Encoding.DER, serialization.PublicFormat.SubjectPublicKeyInfo) for key in signing]
    names = ["vector-initiator", "vector-responder"] if signed else ["", ""]
    identities = public if signed else [b"", b""]
    mode = 2 if signed else 1
    profiles = [canonical_profile(i + 1, mode, context, nonces[i], ephemeral[i], names[i], identities[i]) for i in range(2)]
    wire_profiles = [{1: 2, 2: {i + 1: {}}, 3: {mode: {}}, 4: context, 5: nonces[i], 6: ephemeral[i], 7: names[i], 8: identities[i]} for i in range(2)]
    t0 = digest(label("hello") + lp(profiles[0]) + lp(profiles[1]))
    preimages = [label("signature") + bytes([i + 1]) + t0 for i in range(2)]
    certificates = [{1: {2: {}}, 2: names[i], 3: public[i], 4: signing[i].sign(preimages[i])} if signed else None for i in range(2)]
    if signed:
        for i, cert in enumerate(certificates):
            signing[i].public_key().verify(cert[4], preimages[i])
            try:
                signing[i].public_key().verify(signing[i].sign(t0), preimages[i])
                raise AssertionError("legacy nonce signature accepted")
            except InvalidSignature:
                pass
    auths = [canonical_auth(cert) for cert in certificates]
    t1 = digest(label("auth") + t0 + lp(auths[0]) + lp(auths[1]))
    shared = dh[0].exchange(dh[1].public_key())
    assert shared == dh[1].exchange(dh[0].public_key()) and any(shared)
    try:
        dh[0].exchange(x25519.X25519PublicKey.from_public_bytes(b"\0" * 32))
        raise AssertionError("all-zero shared secret accepted")
    except ValueError:
        pass
    prk = mac(t0, shared)
    finished_keys = [expand(prk, label("finished/" + role) + t1, 32) for role in ["initiator", "responder"]]
    assert finished_keys[0] == HKDF(hashes.SHA3_256(), 32, t0, label("finished/initiator") + t1).derive(shared)
    vi = mac(finished_keys[0], label("verify/initiator") + t1)
    vr = mac(finished_keys[1], label("verify/responder") + t1 + vi)
    t2 = digest(label("session") + t1 + vi + vr)
    secrets = [expand(prk, label("traffic/" + direction) + t2, 32) for direction in ["initiator-to-responder", "responder-to-initiator"]]
    crypto = {"shared_secret_hex": shared.hex(), "prk_hex": prk.hex(), "t0_hex": t0.hex(), "t1_hex": t1.hex(), "t2_hex": t2.hex(), "verify_i_hex": vi.hex(), "verify_r_hex": vr.hex()}
    records = {}
    for i, prefix in enumerate(["i", "r"]):
        direction = i + 1
        key0, iv0 = epoch(secrets[i], t2, direction, 0)
        s1 = next_secret(secrets[i], t2, direction, 1)
        key1, iv1 = epoch(s1, t2, direction, 1)
        for name, value in [("finished_key", finished_keys[i]), ("signature_preimage", preimages[i]), ("secret_0", secrets[i]), ("key_0", key0), ("iv_0", iv0), ("secret_1", s1), ("key_1", key1), ("iv_1", iv1)]:
            crypto[f"{prefix}_{name}_hex"] = value.hex()
        records[prefix + "_data_0"] = record(key0, iv0, t2, direction, 0, 0, 1, ("hello " + prefix).encode())
        records[prefix + "_update_0"] = record(key0, iv0, t2, direction, 0, 1, 2, struct.pack("<Q", 1))
        records[prefix + "_data_1"] = record(key1, iv1, t2, direction, 1, 0, 1, ("更新後 " + prefix).encode())
        records[prefix + "_close_1"] = record(key1, iv1, t2, direction, 1, 1, 3, b"")
    key0, iv0 = epoch(secrets[0], t2, 1, 0)
    wrong_next = record(key0, iv0, t2, 1, 0, 1, 2, struct.pack("<Q", 2))
    premature = record(key0, iv0, t2, 1, 0, 0, 2, struct.pack("<Q", 1))
    data = records["i_data_0"]
    tampered = bytearray.fromhex(data["wire_hex"])
    tampered[-1] ^= 1
    try:
        AESGCM(key0).decrypt(bytes.fromhex(data["nonce_hex"]), bytes(tampered[21:]), bytes.fromhex(data["aad_hex"]))
        raise AssertionError("bad tag accepted")
    except InvalidTag:
        pass
    generation_skip = bytearray.fromhex(data["wire_hex"])
    generation_skip[1:9] = struct.pack("<Q", 2)
    short_ciphertext = bytearray.fromhex(data["wire_hex"])
    short_ciphertext[17:21] = struct.pack("<I", 15)
    negatives = [
        {"name": "bad_tag", "before": [], "wire_hex": tampered.hex(), "error": "Authentication"},
        {"name": "generation_skip", "before": [], "wire_hex": generation_skip.hex(), "error": "Generation"},
        {"name": "sequence_repeat", "before": ["i_data_0"], "wire_hex": data["wire_hex"], "error": "Sequence"},
        {"name": "invalid_ciphertext_length", "before": [], "wire_hex": short_ciphertext.hex(), "error": "Length"},
        {"name": "update_without_data", "before": [], "wire_hex": premature["wire_hex"], "error": "UpdateWithoutData"},
        {"name": "update_wrong_generation", "before": ["i_data_0"], "wire_hex": wrong_next["wire_hex"], "error": "NextGeneration"},
        {"name": "truncated_record", "before": [], "wire_hex": data["wire_hex"][:-2], "error": "UnexpectedEof"},
    ]
    return {
        "mode": "Mutual" if signed else "Anonymous",
        "inputs": {"context_hex": context.hex(), "dh_private_i_hex": dh_seeds[0].hex(), "dh_private_r_hex": dh_seeds[1].hex(), "signing_seed_i_hex": sign_seeds[0].hex(), "signing_seed_r_hex": sign_seeds[1].hex(), "nonce_i_hex": nonces[0].hex(), "nonce_r_hex": nonces[1].hex(), "name_i": names[0], "name_r": names[1]},
        "canonical": {"profile_i_hex": profiles[0].hex(), "profile_r_hex": profiles[1].hex(), "auth_i_hex": auths[0].hex(), "auth_r_hex": auths[1].hex()},
        "handshake_wire": {"profile_i_hex": (b"OMNISC2\0" + frame(wire_profiles[0])).hex(), "profile_r_hex": (b"OMNISC2\0" + frame(wire_profiles[1])).hex(), "auth_i_hex": frame({1: certificates[0]} if signed else {}).hex(), "auth_r_hex": frame({1: certificates[1]} if signed else {}).hex(), "finished_i_hex": frame({1: vi}).hex(), "finished_r_hex": frame({1: vr}).hex()},
        "crypto": crypto, "records": records, "negative_records": negatives,
        "legacy_signatures_hex": [key.sign(t0).hex() for key in signing] if signed else [],
    }


def vectors():
    ghash_bound = 2**26 + 7 * 2**20
    assert ghash_bound < 2**27
    return {"format": "omni-secure-v2-vectors-1", "reference_dependency": "cryptography==50.0.2", "ghash_max_blocks_upper_bound": ghash_bound, "cases": [make_case(True), make_case(False)]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", type=Path)
    parser.add_argument("--check", type=Path)
    args = parser.parse_args()
    value = vectors()
    if args.check:
        assert json.loads(args.check.read_text()) == value, "fixture mismatch"
        print("V2 fixtures match; HKDF, signatures, DH and AES-GCM checks passed")
    elif args.write:
        args.write.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
        print(f"Wrote {args.write}")
    else:
        print(json.dumps(value, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
