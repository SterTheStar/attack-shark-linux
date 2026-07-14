pkgname="attack-shark"
pkgver="1.0.1"
pkgrel="1"
pkgdesc="Userspace driver for Attack Shark mice"
arch=("x86_64")
depends=("libusb")
makedepends=("cargo" "rust" "git")
url="https://github.com/xb-bx/attack-shark-r1-driver"
source=("git+$url")
md5sums=("SKIP")

build() {
    cd "$srcdir/attack-shark-r1-driver"
    make release
}
package() {
    cd "$srcdir/attack-shark-r1-driver"
    DESTDIR="${pkgdir}/" make install
}
