<div align="center">
  <br>
  <img src="res/icons/hicolor/scalable/apps/page.codeberg.sungsphinx.Examine.svg" width="150" />
  <h1>Examine</h1>

  <p>A system information viewer for the COSMIC™ Desktop</p>

  <img src="res/screenshots/distribution.png"/>
</div>

## Install

To install Examine, you will need [just](https://github.com/casey/just), if you're on Pop!\_OS, you can install it with the following command:

```sh
sudo apt install just
```

On Fedora (or derivatives), you can install it with the following command:
```sh
sudo dnf install just
```

After you install it, you can run the following commands to build and install your application:

```sh
just build-release
sudo just install
```

To uninstall simply run

```sh
sudo just uninstall
```
