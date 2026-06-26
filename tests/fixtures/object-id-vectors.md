# Object ID vectors

Current formula:

```text
SHA256("PRIKK-OBJECT-ID-v1" || u16be(type) || u32be(schema_version) || u64be(len(payload)) || payload)
```

## Vector 1

- type: PATCH = `0x0001`
- schema_version: `0x00000001`
- canonical_payload: ASCII `payload`
- payload length: `0x0000000000000007`
- expected ObjectId: `5f8711b3f84991d60b65221d66ed5ec260d28cc19c5c4ed3c1fe44d334265fe6`
