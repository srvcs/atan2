# srvcs-atan2

The atan2 primitive of the srvcs.cloud distributed standard library.

Its single concern: **the four-quadrant arctangent** of `y / x`, in radians
(`atan2(y, x)`). It does not validate input itself — it delegates "is this a
number" to [`srvcs-isnumber`](https://github.com/srvcs/isnumber) over HTTP, the
single source of truth for that question, once per operand.

`srvcs-atan2` is a **floating-point service**: its `result` is an `f64` (a JSON
number that may have a fractional part). Both integers and floats are valid
input — they are coerced to `f64` — so `atan2(1, 1) == 0.7853981633974483` and
`atan2(0, 1) == 0`.

If `srvcs-isnumber` is unreachable, `srvcs-atan2` reports itself **degraded
(503)** rather than guessing.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Compute `atan2(y, x)` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"y": 1, "x": 1}'
# {"y":1,"x":1,"result":0.7853981633974483}
```

Responses:

- `200 {"y": n, "x": n, "result": float}` — evaluated.
- `422` — an operand is not a number (per `srvcs-isnumber`).
- `503` — a dependency is unavailable.

## Dependencies

- [`srvcs-isnumber`](https://github.com/srvcs/isnumber) — input validation.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ISNUMBER_URL` | `http://127.0.0.1:8081` | Base URL of `srvcs-isnumber` |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Orchestration tests stand up a mock `srvcs-isnumber` in-process (one that
genuinely computes "is this a number" from the request body), so the suite runs
without the rest of the fleet. See
[`srvcs/platform`](https://github.com/srvcs/platform) for the shared standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
