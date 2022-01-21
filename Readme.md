# electrumd

Utility to run a regtest electrum wallet daemon process, useful in integration testing environment.

```rust
let electrumd = electrumd::ElectrumD::new(electrumd::downloaded_exe_path().unwrap()).unwrap();
assert_eq!(0, bitcoind.client.get_blockchain_info().unwrap().blocks);
```

Forked from [@RCasatta's `bitcoind`](https://github.com/RCasatta/bitcoind).