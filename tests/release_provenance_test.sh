#!/usr/bin/env bash
# Integration test for release provenance & embedded auditable metadata verification

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

FIXTURE_DIR="${REPO_ROOT}/tests/fixtures/release_provenance"

echo "=== Running Release Provenance Regression Tests ==="

# Python verification script
check_binary_provenance() {
  local target_file="$1"
  python3 - "$target_file" << 'EOF'
import sys
import struct
import zlib
import os

AUDITABLE_SECTIONS = (
    '.dep-v0',
    '.dep-spec',
    '__dep_v0',
    '__dep_spec',
    '.dep_spec',
    '.dep_v0',
)

def parse_elf(data):
    if not data.startswith(b'\x7fELF'):
        return None
    ei_class = data[4]  # 1=32bit, 2=64bit
    ei_data = data[5]   # 1=LE, 2=BE
    endian = '<' if ei_data == 1 else '>'

    sections = {}
    if ei_class == 2: # 64-bit ELF
        e_shoff = struct.unpack(endian + 'Q', data[40:48])[0]
        e_shentsize, e_shnum, e_shstrndx = struct.unpack(endian + 'HHH', data[58:64])
        if e_shstrndx >= e_shnum:
            return None
        shstr_hdr = data[e_shoff + e_shstrndx * e_shentsize : e_shoff + (e_shstrndx + 1) * e_shentsize]
        shstr_offset, shstr_size = struct.unpack(endian + 'QQ', shstr_hdr[24:40])
        strtab = data[shstr_offset : shstr_offset + shstr_size]

        for i in range(e_shnum):
            sh = data[e_shoff + i * e_shentsize : e_shoff + (i + 1) * e_shentsize]
            sh_name = struct.unpack(endian + 'I', sh[0:4])[0]
            sh_offset, sh_size = struct.unpack(endian + 'QQ', sh[24:40])
            name = strtab[sh_name:].split(b'\x00')[0].decode('latin1', errors='ignore')
            sections[name] = data[sh_offset : sh_offset + sh_size]
    elif ei_class == 1: # 32-bit ELF
        e_shoff = struct.unpack(endian + 'I', data[28:32])[0]
        e_shentsize, e_shnum, e_shstrndx = struct.unpack(endian + 'HHH', data[46:52])
        if e_shstrndx >= e_shnum:
            return None
        shstr_hdr = data[e_shoff + e_shstrndx * e_shentsize : e_shoff + (e_shstrndx + 1) * e_shentsize]
        shstr_offset, shstr_size = struct.unpack(endian + 'II', shstr_hdr[16:24])
        strtab = data[shstr_offset : shstr_offset + shstr_size]

        for i in range(e_shnum):
            sh = data[e_shoff + i * e_shentsize : e_shoff + (i + 1) * e_shentsize]
            sh_name = struct.unpack(endian + 'I', sh[0:4])[0]
            sh_offset, sh_size = struct.unpack(endian + 'II', sh[16:24])
            name = strtab[sh_name:].split(b'\x00')[0].decode('latin1', errors='ignore')
            sections[name] = data[sh_offset : sh_offset + sh_size]
    return sections

def parse_macho(data):
    if len(data) < 8:
        return None
    magic = struct.unpack('<I', data[0:4])[0]
    if magic not in (0xfeedfacf, 0xcffaedfe, 0xfeedface, 0xcefaedfe):
        return None
    endian = '<' if magic in (0xfeedfacf, 0xfeedface) else '>'
    is_64 = magic in (0xfeedfacf, 0xcffaedfe)

    sections = {}
    ncmds, sizeofcmds = struct.unpack(endian + 'II', data[16:24])
    offset = 32 if is_64 else 28

    for _ in range(ncmds):
        if offset + 8 > len(data):
            break
        cmd, cmdsize = struct.unpack(endian + 'II', data[offset:offset+8])
        if cmd in (0x1, 0x19):
            if is_64:
                nsects = struct.unpack(endian + 'I', data[offset+64:offset+68])[0]
                sect_offset = offset + 72
                for _ in range(nsects):
                    if sect_offset + 80 > len(data):
                        break
                    sectname = data[sect_offset:sect_offset+16].rstrip(b'\x00').decode('latin1', errors='ignore')
                    s_offset, s_size = struct.unpack(endian + 'QQ', data[sect_offset+48:sect_offset+64])
                    sections[sectname] = data[s_offset:s_offset+s_size]
                    sect_offset += 80
            else:
                nsects = struct.unpack(endian + 'I', data[offset+48:offset+52])[0]
                sect_offset = offset + 56
                for _ in range(nsects):
                    if sect_offset + 68 > len(data):
                        break
                    sectname = data[sect_offset:sect_offset+16].rstrip(b'\x00').decode('latin1', errors='ignore')
                    s_offset, s_size = struct.unpack(endian + 'II', data[sect_offset+36:sect_offset+44])
                    sections[sectname] = data[s_offset:s_offset+s_size]
                    sect_offset += 68
        offset += cmdsize
    return sections

def parse_pe(data):
    if not data.startswith(b'MZ') or len(data) < 0x40:
        return None
    pe_offset = struct.unpack('<I', data[0x3c:0x40])[0]
    if pe_offset + 24 > len(data) or data[pe_offset:pe_offset+4] != b'PE\x00\x00':
        return None
    num_sections = struct.unpack('<H', data[pe_offset+6:pe_offset+8])[0]
    opt_header_size = struct.unpack('<H', data[pe_offset+20:pe_offset+22])[0]
    sect_table_offset = pe_offset + 24 + opt_header_size

    sections = {}
    for i in range(num_sections):
        s_hdr = data[sect_table_offset + i*40 : sect_table_offset + (i+1)*40]
        if len(s_hdr) < 40:
            break
        name = s_hdr[0:8].rstrip(b'\x00').decode('latin1', errors='ignore')
        vsize, raw_size, raw_ptr = struct.unpack('<III', s_hdr[8:20])
        sections[name] = data[raw_ptr : raw_ptr + raw_size]
    return sections

def check_provenance(filepath):
    if not os.path.isfile(filepath):
        print(f"ERROR: File not found: {filepath}", file=sys.stderr)
        return False

    with open(filepath, 'rb') as f:
        data = f.read()

    sections = parse_elf(data)
    if sections is None:
        sections = parse_macho(data)
    if sections is None:
        sections = parse_pe(data)
    if sections is None:
        sections = {}

    found_sec = None
    sec_data = b''
    for sec_name in AUDITABLE_SECTIONS:
        if sec_name in sections:
            found_sec = sec_name
            sec_data = sections[sec_name]
            break

    if not found_sec:
        for candidate in (b'.dep-v0', b'.dep-spec', b'__dep_spec', b'.dep_spec', b'__dep_v0'):
            if candidate in data:
                found_sec = candidate.decode('ascii')
                idx = data.find(candidate)
                for j in range(idx, min(idx + 1024, len(data))):
                    if data[j] == 0x78 and j + 2 <= len(data):
                        try:
                            dec = zlib.decompress(data[j:])
                            if b'packages' in dec or b'dependencies' in dec:
                                sec_data = data[j:]
                                break
                        except Exception:
                            pass
                break

    if found_sec and sec_data:
        try:
            decompressed = zlib.decompress(sec_data)
            if b'packages' in decompressed or b'dependencies' in decompressed:
                print(f"[PASS] File '{filepath}' contains valid auditable metadata in section '{found_sec}'.")
                return True
        except Exception as e:
            print(f"[WARN] Found section '{found_sec}' in '{filepath}' but failed to decompress: {e}")

    print(f"\n[ERROR] Release provenance check FAILED for: {filepath}")
    print("Reason: Embedded auditable dependency metadata section (.dep-v0 / .dep-spec / __dep_spec / .dep_spec) is missing or invalid.\n")
    print("Remediation Guidance for Maintainers:")
    print("  1. Ensure 'auditable = true' is configured under [dist] in 'dist-workspace.toml'.")
    print("  2. Ensure release builds use 'cargo-auditable' (e.g. 'cargo auditable build --release' or 'dist build').")
    print("  3. Verify build scripts / CI workflows have not stripped or omitted the '.dep-v0' section.\n")
    return False

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: provenance_checker.py <binary_file>")
        sys.exit(1)
    ok = check_provenance(sys.argv[1])
    sys.exit(0 if ok else 1)
EOF
}

# 1. Test Auditable Release Binary Fixture (Should PASS)
echo "[TEST] Validating provenance check against auditable release binary fixture..."
if check_binary_provenance "${FIXTURE_DIR}/auditable_release.elf"; then
  echo "[PASS] Auditable release binary fixture successfully passed provenance check."
else
  echo "ERROR: Auditable release binary fixture unexpectedly failed provenance check!"
  exit 1
fi

# 2. Test Ordinary Cargo Binary Fixture (Should FAIL)
echo "[TEST] Validating provenance check fails on ordinary Cargo binary fixture..."
ORDINARY_LOG=$(mktemp)
if check_binary_provenance "${FIXTURE_DIR}/ordinary_cargo.elf" > "${ORDINARY_LOG}" 2>&1; then
  echo "ERROR: Ordinary Cargo binary fixture unexpectedly passed provenance check!"
  rm -f "${ORDINARY_LOG}"
  exit 1
else
  echo "[PASS] Ordinary Cargo binary fixture correctly failed provenance check."
  # Assert remediation guidance was provided
  grep -q "Remediation Guidance for Maintainers" "${ORDINARY_LOG}" || {
    echo "ERROR: Missing maintainer remediation guidance in error output!"
    rm -f "${ORDINARY_LOG}"
    exit 1
  }
  grep -q "auditable = true" "${ORDINARY_LOG}" || {
    echo "ERROR: Missing 'auditable = true' reference in maintainer remediation guidance!"
    rm -f "${ORDINARY_LOG}"
    exit 1
  }
  echo "[PASS] Error message contains maintainer remediation guidance and configuration hints."
fi
rm -f "${ORDINARY_LOG}"

# 3. Test dist-workspace.toml configuration check
echo "[TEST] Validating dist-workspace.toml has 'auditable = true' configured..."
if grep -q "auditable = true" "${REPO_ROOT}/dist-workspace.toml"; then
  echo "[PASS] 'auditable = true' is properly set in dist-workspace.toml."
else
  echo "ERROR: 'auditable = true' missing from dist-workspace.toml!"
  exit 1
fi

echo "=== All Release Provenance Tests PASSED Successfully ==="
