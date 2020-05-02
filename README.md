# OSCQuery in Rust

* [OSCQueryProposal](https://github.com/Vidvox/OSCQueryProposal)
* [OSCSpec 1.0](http://opensoundcontrol.org/spec-1_0)
* [rosc](https://docs.rs/rosc/0.3.0/rosc/) OSC in pure Rust
* [Rust](https://www.rust-lang.org/)
* [websocket example with hyper](https://github.com/websockets-rs/rust-websocket/blob/master/examples/hyper.rs)
* [mdns](https://github.com/librespot-org/libmdns/blob/master/examples/register.rs)

## Node Query Parameters

Only one query parameter is allowed per request

* `VALUE` : eg `http://localhost:3000/foo/bar?VALUE` -> `{"VALUE": [0.5]}`
* `TYPE`
* `RANGE`
* `CLIP_MODE`
* `ACCESS`
* `DESCRIPTION`

## Global Query Parameters

`HOST_INFO`

