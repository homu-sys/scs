pkgname=scs
pkgver=0.1.0
pkgrel=1
pkgdesc="A very cool TUI clock"
arch=('x86_64' 'aarch64')
url="https://github.com/homu-sys/scs"
license=('MIT')
depends=('gcc-libs')
makedepends=('rust')
options=('!strip' '!debug')
source=("scs-$pkgver.tar.xz::https://github.com/homu-sys/scs/releases/download/v$pkgver/scs-$pkgver.tar.xz")
sha256sums=('f5aa23b8f909c8c429fe2f93aefe7a2f0568bb5a74167f0d121cbcfdb9d147c7')

build() {
  cd "$srcdir/$pkgname-$pkgver"
  cargo build --release --locked
}

check() {
  cd "$srcdir/$pkgname-$pkgver"
  cargo test --release --locked
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
