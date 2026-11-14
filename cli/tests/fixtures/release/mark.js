// npm ci が走ったこと（package.json の postinstall）を tests/release.rs に知らせる
require('fs').writeFileSync('npm-ci-ran', '');
