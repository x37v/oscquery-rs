# OSCQuery in Rust

* [OSCQueryProposal](https://github.com/Vidvox/OSCQueryProposal)
* [OSCSpec 1.0](http://opensoundcontrol.org/spec-1_0)
* [rosc](https://docs.rs/rosc/0.3.0/rosc/) OSC in pure Rust
* [Rust](https://www.rust-lang.org/)
* [mdns](https://github.com/librespot-org/libmdns/blob/master/examples/register.rs)

## TODO

* figure out how we can construct graph nodes in the osc callback
  * currently will deadlock, must push a task or command to do it in another thread/context.
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

