<div align="center">

# Ode

<img alt="версия" src="https://img.shields.io/badge/version-0.1.0-4fb3ff?style=for-the-badge&labelColor=0f131b">
<img alt="лицензия" src="https://img.shields.io/badge/license-MIT-9fe8a0?style=for-the-badge&labelColor=0f131b">
<img alt="GTK4" src="https://img.shields.io/badge/GTK4-4fb3ff?style=for-the-badge&labelColor=0f131b&logo=gtk&logoColor=white">
<img alt="libadwaita" src="https://img.shields.io/badge/libadwaita-4fb3ff?style=for-the-badge&labelColor=0f131b">
<img alt="Rust backend" src="https://img.shields.io/badge/Rust_backend-f0a45a?style=for-the-badge&labelColor=0f131b&logo=rust&logoColor=white">

[English](README.md) · [Русский](README.ru.md)

</div>

### Настольное приложение для чтения текстов песен

Ode ищет песни в Genius и показывает текст в отдельном окне. Написано на Rust с
GTK4 и libadwaita, поэтому приложение следует системной теме, шрифтам и языку.

# Возможности

- 🔎 **Поиск.** Введите запрос и получите список песен и исполнителей с обложками и подписями. Открыть можно только песню, исполнители и альбомы показаны для справки.
- 📖 **Текст.** Показывается только текст песни без отвлекающих элементов.
- 💡 **Аннотации.** Аннотации Genius прикреплены к строкам, которые они объясняют.
- 🌍 **Уважает систему.** Поддерживаются английский и русский языки; темы, шрифты и остальные настройки берутся из системы.
- 💾 **Кэш.** Ответы на запросы песен и поиска хранятся на диске час, повторные открытия происходят мгновенно. Размер кэша ограничен, при переполнении первыми удаляются самые старые записи.

# Требования

Ode работает через [Genius API](https://docs.genius.com/) и требует подключения
к интернету. Учётная запись не нужна. Тексты песен принадлежат их правообладателям
и показываются courtesy of Genius.

# Сборка

Для сборки нужны dev-пакеты GTK4, libadwaita и WebKitGTK.

```bash
# Debian / Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev libwebkit2gtk-4.1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel webkit2gtk4.1-devel
# ALT Linux
su -
apt-get install libgtk4-devel libadwaita-devel libwebkit2gtk4.1-devel
```

Проверка зависимостей: `pkg-config --exists gtk4 libadwaita-1`.

Сборка и проверки:

```bash
GENIUS_ACCESS_TOKEN=your_token_here cargo build --release
cargo clippy --all-targets
cargo test
```

Токен можно положить в файл `.env` в корне проекта строкой
`GENIUS_ACCESS_TOKEN=...`. Его читает `build.rs`, в репозиторий файл не попадает.
Без токена приложение собирается и запускается, но запросы к API завершаются
ошибкой, а на странице поиска показывается сообщение «Токен не задан».

# Лицензия

Ode распространяется на условиях **MIT**, см. [LICENSE](LICENSE).

Проект не аффилирован с Genius. Использование API регулируется
[условиями Genius API](https://docs.genius.com/).
