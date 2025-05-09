# Iroh and iroh-blobs workshop for JOnTheBeach 2025

The workshop is structured into multiple exercises. Each exercise is a
self-contained example and can be run using cargon run -p <lessonname>.

There is a branch `diy` where the important code is removed, and you can try
to code the exercises. But is also fine to just use `main` and follow along.

<b>Note: the workshop is using an *alpha* version of iroh-blobs.</b>

To inspect the
docs, run the following command:

```
cargo doc -p iroh-blobs --open
```

The stable version of iroh-blobs on <crates.io> has most of the same features, but
the API has been restructured in preparation of releasing iroh-blobs 1.0.

# Echo 1

A simple echo service, to show the basics of iroh connections

```
cargo run -p echo1
```

# Echo 2

Echo service from before, but done as a iroh protocol handler

```
cargo run -p echo2
```

# Sendme 1

Uses iroh-blobs to send a single file, done as an iroh protocol handler

```
cargo run -p sendme1
```

# Sendme 2

Uses iroh-blobs to send a directory, done as a protocol handler

```
cargo run -p sendme2
```

# Sendfile 3

Receive from multiple senders at the same time.

```
cargo run -p sendme3
```

# Sendfile 4

Publish to a content discovery service, and use that service to find providers
for the content on the receive side


```
cargo run -p sendme3
```
