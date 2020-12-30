# OSCQuery in Rust

* [OSCQueryProposal](https://github.com/Vidvox/OSCQueryProposal)
* [OSCSpec 1.0](http://opensoundcontrol.org/spec-1_0)
* [rosc](https://docs.rs/rosc/0.3.0/rosc/) OSC in pure Rust
* [Rust](https://www.rust-lang.org/)
* [mdns](https://github.com/librespot-org/libmdns/blob/master/examples/register.rs)

## TODO

* Simplify, Document better
* HTTP upgrade to WS
* Allow for control of OSC/WS/HTTP I/O timing
	* So that we can control when our Get/Set etc is accessed
* OSC wildcards
* client impl
  * mirror option
* extended type
* overloads
* tags
* critical?

## Node Query Parameters

Only one query parameter is allowed per request

* `VALUE` : eg `http://localhost:3000/foo/bar?VALUE` -> `{"VALUE": [0.5]}`
* `TYPE`
* `RANGE`
* `CLIPMODE`
* `ACCESS`
* `DESCRIPTION`

## Global Query Parameters

`HOST_INFO`

