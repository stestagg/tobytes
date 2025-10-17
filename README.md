# tobytes-py
A msgpack based serialization/deserialization library

## Glossary

 - *implementer*/*implementation* - an implementation of the tobytes spec
 - *user* - A person/system/organisation that uses an implementation of the tobytes spec
 - *client* - Software that uses an implementation of the tobytes spec
 - *message* - a sequence of bytes that represents a tobytes serialized object
 - *object* - a data structure that can be serialized/deserialized by a library
 - *extension* - An id in the valid range of msgpack extension IDs that is used to encode tobytes specific non-msgpack-native types.

## Spec

tobytes is a specification for encoding data in environments where readers/writers can share pre-arranged libraries of custom types, and have the ability for those types to be serialized/deserialized transparently and correctly.

data is encoded/decoded using msgpack. Any use of msgpack extensions is disallowed, except for as defined in this spec.

Any msgpack message that does not use extensions is a valid tobytes message, and will be encoded/decoded as per the underlying msgpack library and/or specification.

Custom types are any data structure that may not necessarily be natively supported by msgpack libraries. E.g. dates/times, classes, complex numbers, etc. but which can be represented in a sequence of bytes.

The specification does not define how custom types are encoded within their msgpack wrapper, but implementers MUST allow for custom types that encode/decode objects as nested tobytes messages themselves.

Custom types are organised into namespaces. A custom type namespace is a unique string identifier that groups a set of custom types. This allows for clients to negotiate/validate custom type support, and avoid id collisions between different libraries. The use of versioning within namespace identifiers is RECOMMENDED but not specified here.

It is up to the user to ensure that custom type namespaces are unique, and that each client agrees on the IDs and formats of the custom types within those namespaces. 

### Notation

Within msgpack, an extension (ext format family) is a structure that encodes a type integer (0-127) and a byte array payload. tobytes typically encodes structured information inside the payload as a serialized stream of tobytes messages.

This is represented in this spec as:
[Ext <id>, <field1>, <field2>, ...]

<field> describes a type that can be msgpack/tobytes encoded and may be followed by a name.
`any` used here indicates a variable length byte sequence that contains a tobytes message.

Where non-tobytes messages are encoded, the value is prefixed with '!' and an appropriate description of the format is given.

#### Examples:

[Ext 1, uint id, string name]
An extension with id 1, the payload is a msgpack uint, followed by a msgpack string.
This could be encoded as the following byte sequence:
[
    0xd6, # fixext 4
    0x01, # ext id 1
    0x2a, # uint 42 = id
    0xa2, # str len 2
    0x68, 0x69 # 'hi' = name
]

[Ext 2, !u8 len, !u8[len] data]
An extension with id 2, the payload is a u8 length-prefixed byte array.
This could be encoded as the following byte sequence:
[
    0xd6, # fixext 4
    0x02, # ext id 2
    0x03, # u8 len = 3
    0x01, 0x02, 0x03 # byte array = data
]


### Extensions

#### Intern tables

Extension id: 0x06
Format: `[Ext 6, array interned_objects, any data]`

Typically an intern table forms the entire message (but may also be included at any point within the message). The `data` portion of this value represents the content of the value when decoded, while the `interned_objects` array aids decoding.

The contents of an intern table is a msgpack array, followed by a msgpack message:

* `interned_objects`: msgpack array of objects to be interned; the index of each object in this array is its intern ID.
* `data`: msgpack message where objects may be replaced with intern references.

Within `data`, extension id 0x06 is repurposed to represent an intern reference with the format:

* `[Ext 6, uint]` — where `uint` is the intern ID of the referenced object.

**Scoping and nesting.** Intern IDs are scoped to the nearest enclosing intern table. It is not possible (by design) for an intern table to contain another intern table. When multiple intern tables are needed, implementers MUST either (a) merge/flatten them into a single intern table, or (b) embed the object containing the nested intern table using a custom type (see below).

**Acyclicity and ordering.** The `interned_objects` array MUST represent an acyclic dependency graph. An entry MAY contain intern references, but **only to earlier entries** (lower indices). Encoders MUST topologically order `interned_objects` to satisfy this property. Decoders MUST treat any reference to the same or a later index as an error.

During deserialization, when an intern table is encountered, the implementer MUST read the `interned_objects` array (in order) and make each decoded entry available for subsequent references in the same table. When an intern reference is encountered, the implementer MUST replace it with the corresponding object from the intern table.

During serialization, the implementer MAY build an intern table of objects to be interned. When an object that is in the intern table is encountered, the implementer MUST replace it with an intern reference. User/caller preference SHOULD be taken into account when deciding whether to use an intern table. It MAY be beneficial to consider if including an intern table will reduce the overall message size, dynamically, based on the comparative cost of encoding vs message size.

Object equality for the purpose of interning is implementation-defined, but where there is the potential for ambiguity, implementers SHOULD provide a way for users to define equality semantics.

Self-referential objects (e.g. circular references / `interned_objects` entries that contain references to themselves) are not allowed and SHOULD be identified as an error during serialization.

An `interned_objects` entry SHOULD never just be a reference to another interned object; implementers MAY identify this as an error during serialization, but MUST allow it during deserialization (provided it references an earlier entry).


#### Custom Type namespace ID

Extension id: 0x07

Format: [Ext 7, string namespace, uint id, any data]

A custom type namespace id is a mapping from a custom type namespace to an integer ID. The sole purpose of this extension is to allow implementers to reduce the size of messages by replacing string namespace identifiers with integer IDs.

Say a user has chosen a long namespace id: 'com.example.group.team.mylibrary.v1.common', if multiple objects are from this namespace are being serialized in a message, it may be beneficial to replace the string namespace with an integer id.

the `data` field contains a message, within which any custom type namespace reference using the string namespace MAY be replaced with the integer id.

The mapping from namespace to id is only valid within the `data` value, and does not persist or apply to any value outside of it. If a custom type namespace id mapping is nested within another custom type namespace id mapping, and both mappings define the same namespace with different IDs, the inner mapping takes precedence within its `data` value.

#### Custom Types

Extension id: 0x08
Format: [Ext 8, string namespace / uint namespace_id, uint type_id, !bytes data]

A serialized custom type. The `namespace` field is either a string namespace or an integer namespace_id (as defined above). The `type_id` field is the custom type id within that namespace. The `data` field contains the serialized representation of the object.

The data field MAY contain nested tobytes messages, including other custom types, or may be any other serialized representation as defined by the user.  Implementers MUST provide a way for users to include nested tobytes messages within `data`.

#### Unknown Namespaces/Types

 - **Unknown namespace** - The library MUST allow the user to configure how unknown namespaces are handled. The default behaviour SHOULD be to raise an error during deserialization, but it must be possible for the user to decode messages containing unknown namespaces by treating custom types within those namespaces as raw byte arrays or a specific placeholder type containing the raw bytes and associated namespace/type information.
 - **Unknown namespace ID** - An integer namespace ID that is not contained within a surrounding custom type namespace ID mapping MUST be treated as an error during deserialization.
 - **Unknown type ID with known namespace** - The library SHOULD allow the user to configure how unknown type IDs are handled, but MAY unconditionally treat this situation as an error

### Implementations

 - SHOULD allow the user to configure intern behaviour and Custom Type namespace ID substitution
 - MUST provide a way for the user to register custom types within namespaces, and provide serialization/deserialization functions for those types ( encode: object -> bytes, decode: bytes -> object )
 - SHOULD provide users with the set of registered custom type namespaces that are available at any point
 - MUST provide a way for users to load/register their own custom type namespaces.
 - MAY allow for complex use cases where the encoding/decoding of *any* type_id within a namespace is handled by a single function, with the type_id passed as an argument to the function.
 - MAY provide a built-in or default custom type namespace
