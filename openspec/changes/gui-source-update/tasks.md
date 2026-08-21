# Tasks

- [x] 1. backend: `run_update` Tauri command (async, spawn_blocking で core
      `update(repo, &StateStore::default(), &tc, true)` を capture mode で
      実行。昇格なし。`generate_handler!` へ登録)
- [x] 2. frontend: index.html に「ソース更新」ボタン (id: `update`) を追加、
      main.js に event binding (`run(() => invoke("run_update"), ...)`) と
      成功後の dashboard 再取得、`refresh()` での managed source による
      「アップグレード」ボタン隠蔽 gate を実装
- [x] 3. test: desktop の unit test に 3-layer regression test を追加
      (main.js の `run_update` invoke 参照 + upgrade 隠蔽 gate + DOM id
      `update` / `upgrade` の存在)。`rustfmt --edition 2021 --check` で
      parse 検証
- [x] 4. 検証: `cargo test` (core / cli)、desktop は local compile 不可の
      ため rustfmt parse 検証 + `node --check` (main.js)。openspec validate
