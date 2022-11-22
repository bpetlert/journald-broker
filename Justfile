@_default:
  just --list

# Run with local configuration file
run-debug-default:
  RUST_BACKTRACE=1 RUST_LOG=journald_broker=debug cargo run -- --config-file ./journald-broker.toml

# Run with debug log
run-debug +ARGS='':
  RUST_BACKTRACE=1 RUST_LOG=journald_broker=debug cargo run -- {{ARGS}}

# Run test
test +CASES='':
  RUST_BACKTRACE=1 RUST_LOG=bpl_sys=debug cargo test -- {{CASES}}

# Increase semver
bump-version VERSION:
  just _bump-cargo {{VERSION}}
  just _bump-pkgbuild {{VERSION}}
  cargo check

@_bump-cargo VERSION:
  cargo bump {{VERSION}}

@_bump-pkgbuild VERSION:
  sed -i -e "s/pkgver=.*/pkgver={{VERSION}}/g" -e "s/pkgrel=.*/pkgrel=1/g"  PKGBUILD.local

# Commit bump version and release
release VERSION:
  git add Cargo.lock Cargo.toml PKGBUILD.local
  git commit --message="chore(release): {{VERSION}}"
  git tag --sign --annotate {{VERSION}} --message="version {{VERSION}}" --edit

# Update and audit dependencies
update-deps:
  cargo update
  cargo upgrade
  cargo audit

# Crate Arch package from GIT source
makepkg:
  makepkg -p PKGBUILD.local
  git co PKGBUILD.local
