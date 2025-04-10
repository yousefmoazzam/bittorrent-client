# How to run

The following shows an example downloading the (at the time of writing) latest version Debian
ISO:
```
> wget https://cdimage.debian.org/debian-cd/current/amd64/bt-cd/debian-12.10.0-amd64-netinst.iso.torrent
> RUST_LOG=info cargo run debian-12.10.0-amd64-netinst.iso.torrent debian.iso
...
> wget https://cdimage.debian.org/debian-cd/current/amd64/bt-cd/SHA256SUMS
> cat SHA256SUMS | head -n 1
ee8d8579128977d7dc39d48f43aec5ab06b7f09e1f40a9d98f2a9d149221704a  debian-12.10.0-amd64-netinst.iso
> sha256sum debian.iso
ee8d8579128977d7dc39d48f43aec5ab06b7f09e1f40a9d98f2a9d149221704a  debian.iso
```

# Limitations

- download functionality only, no upload
- single-file download only, no multi-file download
- no [endgame](https://wiki.theory.org/BitTorrentSpecification#End_Game) implementation, so the
  final few blocks often won't download as quickly as they could

# Acknowledgements

I used [this blog post](https://blog.jse.li/posts/torrent/) and the associated [Github
repo](https://github.com/veggiedefender/torrent-client/) as a guide, both were incredibly
helpful.

The [official BitTorrent specification](https://www.bittorrent.org/beps/bep_0003.html) was a
useful reference, as was the [unofficial
specification](https://wiki.theory.org/BitTorrentSpecification) for supplementary information.
