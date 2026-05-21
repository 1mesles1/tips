# Maintainer: measles <your-email@example.com>
pkgname=tips-git
pkgver=0.0.1
pkgrel=1
pkgdesc="A modular Rust TUI knowledge base and snippet manager for the Linux terminal (TTY)"
arch=('x86_64' 'aarch64')
url="https://github.com"
license=('MIT')
depends=('gcc-libs')
makedepends=('rust' 'cargo' 'git')
provides=('tips')
conflicts=('tips')

source=("${pkgname}::git+file://${PWD}")
sha256sums=('SKIP')

pkgver() {
  cd "${srcdir}/${pkgname}"
  printf "0.1.0.r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

prepare() {
  cd "${srcdir}/${pkgname}"

  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "${srcdir}/${pkgname}"

  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --all-features
}

package() {
  cd "${srcdir}/${pkgname}"

  install -Dm755 target/release/tips "${pkgdir}/usr/bin/tips"
  

  install -Dm644 README.md "${pkgdir}/usr/share/doc/${pkgname}/README.md"
}
