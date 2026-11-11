"""Verify a Tauri updater .sig against the pubkey embedded in tauri.conf.json.

Usage: python verify-updater-signature.py <tauri.conf.json> <artifact> <artifact.sig>
Exits non-zero if the signature does not match, so a release signed with a
key that installed launchers do not trust is never published.
"""
import base64
import hashlib
import json
import sys

from nacl.exceptions import BadSignatureError
from nacl.signing import VerifyKey


def minisign_lines(b64_text: str) -> list[str]:
    return base64.b64decode(b64_text.strip()).decode("utf-8").strip().splitlines()


def main() -> int:
    conf_path, artifact_path, sig_path = sys.argv[1:4]
    with open(conf_path, encoding="utf-8") as f:
        pubkey_b64 = json.load(f)["plugins"]["updater"]["pubkey"]
    if not pubkey_b64.strip():
        print("tauri.conf.json has no updater pubkey", file=sys.stderr)
        return 1

    pk = base64.b64decode(minisign_lines(pubkey_b64)[1])
    with open(sig_path, encoding="utf-8") as f:
        sig_lines = minisign_lines(f.read())
    sig = base64.b64decode(sig_lines[1])

    if pk[:2] != b"Ed" or len(pk) != 42 or len(sig) != 74:
        print("malformed key or signature", file=sys.stderr)
        return 1
    if pk[2:10] != sig[2:10]:
        print(
            f"key id mismatch: pubkey {pk[2:10][::-1].hex().upper()} "
            f"vs signature {sig[2:10][::-1].hex().upper()}",
            file=sys.stderr,
        )
        return 1

    with open(artifact_path, "rb") as f:
        data = f.read()
    message = hashlib.blake2b(data, digest_size=64).digest() if sig[:2] == b"ED" else data
    try:
        VerifyKey(pk[10:]).verify(message, sig[10:])
    except BadSignatureError:
        print("signature does not verify against the embedded pubkey", file=sys.stderr)
        return 1
    print(f"OK: {artifact_path} is signed by key {pk[2:10][::-1].hex().upper()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
