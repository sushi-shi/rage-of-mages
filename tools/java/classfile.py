#!/usr/bin/env python3
"""Parse J2ME `.class` files directly from bytes (no decompiler in the loop).

The original class files are the authority (rulebook R2/R8): a decompiler's
formatting, invented local names, and reconstructed expressions must never
become symbol identity or arithmetic shape. This module parses only the
class-file structures this CLDC-era corpus needs — the constant pool, the field
and method tables, and each method's `Code` attribute bytes — and walks the
bytecode instruction stream.

Ported from the proven `gothic-mobile/tools/corpus/classfile.py`, trimmed to the
two consumers this game needs (`validate_numeric_shape.py`, `validate_symbols.py`)
and taught to retain the raw `Code` bytes so callers need not re-walk the class.

A malformed or truncated class raises `ClassFormatError`; callers turn that into
a reported problem rather than a crash (parsers never panic).
"""

from __future__ import annotations

import hashlib
import struct
from dataclasses import dataclass, field
from typing import Any, Iterable


class ClassFormatError(ValueError):
    """A malformed, truncated, or unsupported class file."""


def sha256(data: bytes | str) -> str:
    if isinstance(data, str):
        data = data.encode("utf-8")
    return hashlib.sha256(data).hexdigest()


class Reader:
    """Big-endian byte reader that refuses to read past the end."""

    def __init__(self, data: bytes):
        self.data = memoryview(data)
        self.offset = 0

    def take(self, size: int) -> bytes:
        end = self.offset + size
        if size < 0 or end > len(self.data):
            raise ClassFormatError(
                f"truncated class data at offset {self.offset}: need {size} bytes"
            )
        value = self.data[self.offset:end].tobytes()
        self.offset = end
        return value

    def u1(self) -> int:
        return self.take(1)[0]

    def u2(self) -> int:
        return struct.unpack(">H", self.take(2))[0]

    def u4(self) -> int:
        return struct.unpack(">I", self.take(4))[0]


@dataclass
class Attribute:
    name: str
    data: bytes


@dataclass
class FieldSymbol:
    ordinal: int
    name: str
    descriptor: str
    access_flags: int


@dataclass
class MethodSymbol:
    ordinal: int
    name: str
    descriptor: str
    access_flags: int
    code: bytes | None = None  # None for abstract/native methods (no Code attr)


@dataclass
class ClassInfo:
    member_path: str
    internal_name: str
    access_flags: int
    super_name: str | None
    interfaces: list[str]
    fields: list[FieldSymbol]
    methods: list[MethodSymbol]


class ConstantPool:
    def __init__(self, entries: list[Any]):
        self.entries = entries

    def get(self, index: int) -> Any:
        if index <= 0 or index >= len(self.entries) or self.entries[index] is None:
            raise ClassFormatError(f"invalid constant-pool index {index}")
        return self.entries[index]

    def utf8(self, index: int) -> str:
        entry = self.get(index)
        if entry[0] != "Utf8":
            raise ClassFormatError(f"constant {index} is not UTF-8")
        return entry[1]

    def class_name(self, index: int) -> str:
        entry = self.get(index)
        if entry[0] != "Class":
            raise ClassFormatError(f"constant {index} is not a class")
        return self.utf8(entry[1])


def parse_constant_pool(reader: Reader) -> ConstantPool:
    count = reader.u2()
    entries: list[Any] = [None] * count
    index = 1
    while index < count:
        tag = reader.u1()
        if tag == 1:
            raw = reader.take(reader.u2())
            entries[index] = ("Utf8", raw.decode("utf-8", errors="surrogateescape"))
        elif tag == 3:
            entries[index] = ("Integer", struct.unpack(">i", reader.take(4))[0])
        elif tag == 4:
            entries[index] = ("Float", struct.unpack(">f", reader.take(4))[0])
        elif tag == 5:
            entries[index] = ("Long", struct.unpack(">q", reader.take(8))[0])
            index += 1  # longs occupy two constant-pool slots
        elif tag == 6:
            entries[index] = ("Double", struct.unpack(">d", reader.take(8))[0])
            index += 1
        elif tag == 7:
            entries[index] = ("Class", reader.u2())
        elif tag == 8:
            entries[index] = ("String", reader.u2())
        elif tag in {9, 10, 11}:
            kind = {9: "Fieldref", 10: "Methodref", 11: "InterfaceMethodref"}[tag]
            entries[index] = (kind, reader.u2(), reader.u2())
        elif tag == 12:
            entries[index] = ("NameAndType", reader.u2(), reader.u2())
        elif tag == 15:
            entries[index] = ("MethodHandle", reader.u1(), reader.u2())
        elif tag == 16:
            entries[index] = ("MethodType", reader.u2())
        elif tag in {17, 18}:
            entries[index] = (
                "Dynamic" if tag == 17 else "InvokeDynamic",
                reader.u2(),
                reader.u2(),
            )
        elif tag in {19, 20}:
            entries[index] = ("Module" if tag == 19 else "Package", reader.u2())
        else:
            raise ClassFormatError(f"unsupported constant-pool tag {tag}")
        index += 1
    return ConstantPool(entries)


def parse_attributes(reader: Reader, pool: ConstantPool) -> list[Attribute]:
    attributes = []
    for _ in range(reader.u2()):
        name = pool.utf8(reader.u2())
        attributes.append(Attribute(name, reader.take(reader.u4())))
    return attributes


# Number of operand bytes for opcodes with a fixed operand length. Everything
# not listed here has zero operand bytes. tableswitch/lookupswitch/wide are
# handled specially in `instructions`.
FIXED_OPERANDS = {
    16: 1,   # bipush
    17: 2,   # sipush
    18: 1,   # ldc
    19: 2,   # ldc_w
    20: 2,   # ldc2_w
    **{opcode: 1 for opcode in range(21, 26)},   # iload..aload
    **{opcode: 1 for opcode in range(54, 59)},   # istore..astore
    132: 2,  # iinc
    **{opcode: 2 for opcode in range(153, 169)},  # if* / goto / jsr (branch)
    169: 1,  # ret
    **{opcode: 2 for opcode in range(178, 185)},  # get/put static/field, invoke*
    185: 4,  # invokeinterface
    186: 4,  # invokedynamic
    187: 2,  # new
    188: 1,  # newarray
    189: 2,  # anewarray
    192: 2,  # checkcast
    193: 2,  # instanceof
    197: 3,  # multianewarray
    198: 2,  # ifnull
    199: 2,  # ifnonnull
    200: 4,  # goto_w
    201: 4,  # jsr_w
}


def instructions(code: bytes) -> Iterable[tuple[int, int, bytes]]:
    """Yield (start_offset, opcode, operand_bytes) for each instruction."""
    offset = 0
    while offset < len(code):
        start = offset
        opcode = code[offset]
        offset += 1
        if opcode == 170:  # tableswitch
            offset += (-offset) % 4
            if offset + 12 > len(code):
                raise ClassFormatError(f"truncated tableswitch at {start}")
            low, high = struct.unpack(">ii", code[offset + 4:offset + 12])
            count = high - low + 1
            if count < 0 or count > len(code):
                raise ClassFormatError(f"invalid tableswitch range at {start}")
            offset += 12 + count * 4
        elif opcode == 171:  # lookupswitch
            offset += (-offset) % 4
            if offset + 8 > len(code):
                raise ClassFormatError(f"truncated lookupswitch at {start}")
            count = struct.unpack(">i", code[offset + 4:offset + 8])[0]
            if count < 0 or count > len(code):
                raise ClassFormatError(f"invalid lookupswitch count at {start}")
            offset += 8 + count * 8
        elif opcode == 196:  # wide
            if offset >= len(code):
                raise ClassFormatError(f"truncated wide at {start}")
            offset += 5 if code[offset] == 132 else 3
        else:
            offset += FIXED_OPERANDS.get(opcode, 0)
        if offset > len(code):
            raise ClassFormatError(f"truncated opcode {opcode:#x} at {start}")
        yield start, opcode, code[start + 1:offset]


def _code_bytes(data: bytes) -> bytes:
    """Extract the raw instruction bytes from a `Code` attribute payload."""
    reader = Reader(data)
    reader.u2()  # max_stack
    reader.u2()  # max_locals
    return reader.take(reader.u4())


def parse_member(reader: Reader, pool: ConstantPool, ordinal: int, is_method: bool):
    access_flags = reader.u2()
    name = pool.utf8(reader.u2())
    descriptor = pool.utf8(reader.u2())
    attributes = parse_attributes(reader, pool)
    if not is_method:
        return FieldSymbol(ordinal, name, descriptor, access_flags)
    code_attributes = [a for a in attributes if a.name == "Code"]
    if len(code_attributes) > 1:
        raise ClassFormatError(f"method {name}{descriptor} has >1 Code attribute")
    code = _code_bytes(code_attributes[0].data) if code_attributes else None
    return MethodSymbol(ordinal, name, descriptor, access_flags, code)


def parse_class(member_path: str, data: bytes) -> ClassInfo:
    reader = Reader(data)
    if reader.u4() != 0xCAFEBABE:
        raise ClassFormatError(f"{member_path}: invalid class magic")
    reader.u2()  # minor_version
    reader.u2()  # major_version
    pool = parse_constant_pool(reader)
    access_flags = reader.u2()
    internal_name = pool.class_name(reader.u2())
    super_index = reader.u2()
    super_name = pool.class_name(super_index) if super_index else None
    interfaces = [pool.class_name(reader.u2()) for _ in range(reader.u2())]
    fields = [parse_member(reader, pool, i, False) for i in range(reader.u2())]
    methods = [parse_member(reader, pool, i, True) for i in range(reader.u2())]
    parse_attributes(reader, pool)
    if reader.offset != len(data):
        raise ClassFormatError(
            f"{member_path}: {len(data) - reader.offset} trailing bytes"
        )
    return ClassInfo(
        member_path=member_path,
        internal_name=internal_name,
        access_flags=access_flags,
        super_name=super_name,
        interfaces=interfaces,
        fields=fields,
        methods=methods,
    )
