# Uploading Files

File uploads work by first sending a file to the server and then using the ID provided.

You can find out what kinds of files you can upload by visiting [the API documentation](https://cdn.stoatusercontent.com/scalar).

You must specify session/bot authentication token as with any other API route.

You will receive the following JSON response:

```json
{
  "id": "0"
}
```

You can use the ID wherever a file is required in the API.

## Chunked uploads

Clients should upload files in chunks so uploads continue to work behind CDNs and reverse proxies with request body limits.

The chunk size is advertised by the API root as `features.limits.global.chunk_upload_size`. If it is missing, clients should default to 50 MiB.

For each file:

1. Generate a UUID for the upload.
2. Calculate the SHA-256 checksum of the original file.
3. Split the file into chunks of `chunk_upload_size` bytes.
4. Send each chunk as **POST** `{endpoint}/{tag}/chunks` with `multipart/form-data`.
5. Complete the upload with **POST** `{endpoint}/{tag}/chunks/{upload_id}/complete`.

Each chunk request must include these fields:

| Field | Description |
| :-- | :-- |
| `upload_id` | Client-generated upload UUID. |
| `chunk_index` | Zero-based chunk number. |
| `total_chunks` | Total number of chunks in the file. |
| `total_size` | Original file size in bytes. |
| `chunk` | The chunk data. |

The complete request is JSON:

```json
{
  "filename": "example.png",
  "total_chunks": 3,
  "total_size": 123456789,
  "sha256": "..."
}
```

The server stores chunks temporarily on local disk, combines them after completion, verifies `sha256`, and only then uploads the resulting file to object storage.

Code sample in JavaScript using Fetch API:

```js
const uploadId = crypto.randomUUID();
const chunkSize = configuration.features.limits.global.chunk_upload_size ?? 50 * 1024 * 1024;
const totalChunks = Math.max(1, Math.ceil(file.size / chunkSize));
const hashBuffer = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
const sha256 = [...new Uint8Array(hashBuffer)]
  .map((byte) => byte.toString(16).padStart(2, "0"))
  .join("");

for (let chunkIndex = 0; chunkIndex < totalChunks; chunkIndex++) {
  const offset = chunkIndex * chunkSize;
  const body = new FormData();
  body.set("upload_id", uploadId);
  body.set("chunk_index", String(chunkIndex));
  body.set("total_chunks", String(totalChunks));
  body.set("total_size", String(file.size));
  body.set("chunk", file.slice(offset, offset + chunkSize), file.name);

  await fetch(`${endpoint}/${tag}/chunks`, {
    method: "POST",
    body,
    headers: {
      "X-Session-Token": "...", // or X-Bot-Token
    },
  });
}

const data = await fetch(`${endpoint}/${tag}/chunks/${uploadId}/complete`, {
  method: "POST",
  headers: {
    "Content-Type": "application/json",
    "X-Session-Token": "...", // or X-Bot-Token
  },
  body: JSON.stringify({
    filename: file.name,
    total_chunks: totalChunks,
    total_size: file.size,
    sha256,
  }),
}).then((res) => res.json());

// use data.id
```

## Legacy single request uploads

For compatibility, clients may still send a **POST** to `{endpoint}/{tag}` along with a `multipart/form-data` body with one field `file` that contains the file you wish to upload. This path is still subject to request body limits and should not be used for large files.

```js
const body = new FormData();
body.append("file", file);

const data = await fetch(`${endpoint}/${tag}`, {
  method: "POST",
  body,
  headers: {
    "X-Session-Token": "...", // or X-Bot-Token
  },
}).then((res) => res.json());

// use data.id
```

## Differences from old Autumn

If you are migrating from old Autumn, the following key points are important:

- There are only two paths that serve a unique image, the preview version of it (if available) and the original image.
- You should not specify any query parameters under any circumstance, the preview route will serve the optimal size for the content type.
- Preview routes for banners, emojis, backgrounds, and attachments will redirect to the original file where the file is not an image or the image is animated.
- If you are currently using logic to replace the URL path to start/stop animations, you should use the following templates: (NB. this only applies to avatars and icons)
  - Non-animated file: `/{tag}/{file_id}`
  - Animated file: `/{tag}/{file_id}/{file_name}` or `/{tag}/{file_id}/original` (if name unavailable)
