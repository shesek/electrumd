# electrumd

Utility to run a regtest electrum wallet daemon process, useful in integration testing environment.

```rust
let electrum_wallet = electrumd::ElectrumD::new(electrumd::downloaded_exe_path().unwrap()).unwrap();

let addr = electrum_wallet.call("createnewaddress", &json!([]))
```

Forked from [@RCasatta's `bitcoind`](https://github.com/RCasatta/bitcoind).