# Core (Event Bus/Superclient)
This repository contains two core components of the system - the event bus and the superclient. The event bus is the core of the entire system, it is the single source of truth where all events are processed and distributed throughout the system, it boasts the following features:

  - [Rust](https://www.rust-lang.org/en-US/) application built using an actor architecture (using the [fantastic actix library](https://github.com/actix/actix/)).
  - Provides a websockets API for clients to register, query, acknowledge, observe and produce events.
  - Persists events to [Kafka](https://kafka.apache.org/) as a primary datastore for the events and to [Couchbase](https://www.couchbase.com/) for later querying.
  - Ensures events are processed and re-delivers to services if they are not.
  - Manages consistency/ordering between events without needing knowledge of event contents.
  - Works with multiple instances of services to avoid duplication of processing and allow for horizontal scaling of clients.

The superclient replaced earlier iterations of event bus clients and client libraries by providing a framework for building services in [Lua](https://www.lua.org/). Motivated by a desire to increase productivity and reduce iteration time when working on event bus clients, the superclient vastly decreases complexity of services and the amount of code that needed to be maintained. This application provides a stable API in Lua for handling incoming events, producing new events, registering HTTP routes and re-building state from previous events. It boasts the following features:

  - [Rust](https://www.rust-lang.org/en-US/) application built using an actor architecture (using the [fantastic actix library](https://github.com/actix/actix/)).
  - Handles communication with the event bus using shared validated JSON schemas (thanks to [serde_json](https://github.com/serde-rs/json)) over websockets!
  - Provides a HTTP server for services to advertise REST endpoints for the various operations they provide.
  - Works with [Redis](https://redis.io/) to provide services with persistent storage of state.
  - Dynamically loads and runs Lua scripts (using the excellent [rlua](https://github.com/chucklefish/rlua/)) containing only service business logic allowing for vast code re-use between services and quick iteration.
  - Builds upon event bus functionality to allow easy re-building of entire state from previous events.

## Dependencies
Both the event bus and the superclient have various dependencies - both for compilation and at runtime.

### Compilation
In order for the event bus to interact with Couchbase, it requires that `libcouchbase` be installed on the system, before attempting to build the projects in this repository, please install `libcouchbase` by following the instructions [on the Couchbase documentation](https://developer.couchbase.com/documentation/server/current/sdk/c/start-using-sdk.html).

On Ubuntu systems, this is currently handled by the `./build_deps.sh` script.

### Runtime
There are various runtime dependencies of the event bus and the superclient - the vast majority of these are handled by the included Docker configurations included in this repository:

  1. Run `./start.sh` to add the container names to your `/etc/hosts` file and start the dependant containers (Kafka, Zookeeper, Couchbase, Redis).
  2. Use the event bus and superclient (see below instructions).
  3. Run `./stop.sh` to kill and remove all the running containers.

## How to run
Both the event bus and the superclient are managed by [Cargo](https://github.com/rust-lang/cargo) - Rust's excellent package manager - therefore compilation of dependencies (not including those mentioned above) and executing the application is handled automatically:

  1. (Event bus) Browse to the server directory - `cd server`.
  2. (Event bus) Run `cargo run -- server` to start the event bus.
  3. (Superclient) Start the event bus.
  4. (Superclient) Browse to the service directory - `cd service`.
  5. (Superclient) Run `cargo run -- services/transaction.lua` to start the superclient with the provided Lua file as the current service.

## How to test
Both the event bus and the superclient are managed by [Cargo](https://github.com/rust-lang/cargo) - Rust's excellent package manager - therefore testing of the event bus, superclient and common libraries are handled as follows:

  1. Browse to the directory of the project you wish to test.
  2. Run `cargo test`.

