pkgname=scs
pkgver=0.2.0
pkgrel=1
pkgdesc="A very cool TUI clock"
arch=('x86_64' 'aarch64')
url="https://github.com/homu-sys/scs"
license=('MIT')
depends=('gcc-libs')
makedepends=('rust')
options=('!strip' '!debug')
source=("scs-$pkgver.tar.xz::https://github.com/homu-sys/scs/releases/download/v$pkgver/scs-$pkgver.tar.xz")
sha256sums=('cffbe151eaa56b3a32d02f2b39b2ac8f1fa01850314a1fb44b5a490049f8430b')

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
  install -Dm644 scs.1 "$pkgdir/usr/share/man/man1/scs.1"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
