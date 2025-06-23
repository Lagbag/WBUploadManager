# Менеджер контента Wildberries / Wildberries Content Manager

---

## О проекте / About the Project

**Русский**  
Мощное и удобное настольное приложение, разработанное на языке Rust, для автоматизации загрузки медиаконтента на платформу Wildberries. Приложение позволяет загружать файлы с Яндекс.Диска или локальных директорий, сопоставлять их с артикулами и отправлять на Wildberries через официальный API.

**English**  
A powerful and user-friendly desktop application built in Rust to streamline the process of uploading media content to Wildberries, a leading e-commerce platform. The tool automates file retrieval from Yandex Disk or local directories, matches them with vendor codes, and uploads them to Wildberries using the official API.

---

## Возможности / Features

**Русский**  
- **Управление профилями**: Создание, редактирование и удаление профилей с уникальными API-ключами Wildberries для удобного переключения между аккаунтами.  
- **Гибкий выбор источника файлов**: Поддержка загрузки медиафайлов с Яндекс.Диска с возможностью пакетной обработки.  
- **Обработка артикулов**: Автоматическое сопоставление файлов с артикулами на основе их имен и загрузка в соответствующие карточки товаров на Wildberries.  
- **Обработка ошибок**: Отслеживание и повторная обработка неудачных загрузок с отдельным интерфейсом для повторного запуска ошибочных артикулов.  
- **Кроссплатформенность**: Разработано с использованием Rust и `eframe`, что обеспечивает совместимость с Windows, macOS и Linux.  
- **Современный интерфейс**: Интуитивно понятный интерфейс с поддержкой тёмной и светлой тем, созданный с использованием `egui`.  
- **Без локального хранения**: Ссылки обрабатываются в памяти и отправляются напрямую на Wildberries, минимизируя использование диска.

**English**  
- **Profile Management**: Create, edit, and delete multiple profiles with unique Wildberries API keys for seamless account switching.  
- **Flexible File Sourcing**: Fetch media files from Yandex Disk URLs, with support for batch processing.  
- **Vendor Code Handling**: Automatically associate files with vendor codes based on naming conventions and upload them to the correct Wildberries product listings.  
- **Error Recovery**: Track and retry failed uploads with a dedicated interface for reprocessing erroneous vendor codes.  
- **Cross-Platform**: Built with Rust and `eframe`, ensuring compatibility across Windows, macOS, and Linux.  
- **Modern UI**: Intuitive and responsive interface with dark/light mode support, built using `egui` for a native desktop experience.  
- **No Local Storage**: Links are processed in memory and sent directly to Wildberries, minimizing disk usage.

---

## Начало работы / Getting Started

### Требования / Prerequisites

**Русский**  
- Rust (рекомендуется последняя стабильная версия)  
- Cargo (включён в Rust)  
- API-ключ Wildberries (получите в личном кабинете продавца)  
- Опционально: Публичные ссылки на Яндекс.Диск для загрузки файлов  

**English**  
- Rust (stable, latest version recommended)  
- Cargo (included with Rust)  
- A Wildberries API key (obtain from your Wildberries seller account)  
- Optional: Yandex Disk public links for file sourcing  

### Установка / Installation

**Русский**  
1. Клонируйте репозиторий:  
   ```bash
   git clone https://github.com/your-username/wildberries-content-manager.git
   cd wildberries-content-manager
   ```

2. Соберите проект:  
   ```bash
   cargo build --release
   ```

3. Запустите приложение:  
   ```bash
   cargo run --release
   ```

**English**  
1. Clone the repository:  
   ```bash
   git clone https://github.com/your-username/wildberries-content-manager.git
   cd wildberries-content-manager
   ```

2. Build the project:  
   ```bash
   cargo build --release
   ```

3. Run the application:  
   ```bash
   cargo run --release
   ```

### Использование / Usage

**Русский**  
1. **Запуск приложения**: Откройте приложение для доступа к основному интерфейсу.  
2. **Управление профилями**:  
   - Добавьте новый профиль с уникальным именем и API-ключом Wildberries.  
   - Выберите или удалите существующие профили (должен остаться хотя бы один профиль).  
3. **Указание источника файлов**:  
   - Выберите ссылки на Яндекс.Диск (через запятую) или локальную папку.  
   - Убедитесь, что имена файлов соответствуют шаблону: `<артикул продовца>[_<номер>].<расширение>` (например, `VendorCodeTest1_1.jpg`).  
4. **Ввод артикулов**: Введите артикулы (по одному на строку) для сопоставления с файлами.  
5. **Запуск обработки**: Нажмите «Запуск» для начала загрузки файлов и отправки на Wildberries.  
6. **Обработка ошибок**: Просмотрите и повторите попытку для неудачных артикулов с помощью кнопки «Повторить для ошибочных».  

**English**  
1. **Launch the Application**: Start the app to access the main interface.  
2. **Manage Profiles**:  
   - Add a new profile with a unique name and your Wildberries API key.  
   - Select or delete existing profiles as needed (at least one profile must remain).  
3. **Specify File Source**:  
   - Choose Yandex Disk URLs (comma-separated) or select a local folder.  
   - Ensure files follow the naming convention: `<vendor_code>[_<number>].<extension>` (e.g., `VendorCodeTest1_1.jpg`).  
4. **Enter Vendor Codes**: Input vendor codes (one per line) to match with files.  
5. **Start Processing**: Click "Launch" to begin fetching files and uploading them to Wildberries.  
6. **Handle Errors**: Review and retry any failed vendor codes using the "Retry Failed" button.

---

## Конфигурация / Configuration

**Русский**  
Конфигурация хранится в файле `profiles.json` в системной директории конфигурации (например, `~/.config/com.yandex.downloader/profiles.json` на Linux). В этом файле сохраняются имена профилей и API-ключи.

**English**  
Configuration is stored in a `profiles.json` file located in your system’s configuration directory (e.g., `~/.config/com.yandex.downloader/profiles.json` on Linux). This file saves profile names and API keys.

---

## Структура проекта / Project Structure

**Русский**  
```
├── src
│   ├── app.rs          # Основная логика приложения и интерфейс
│   ├── config.rs       # Управление конфигурацией
│   ├── downloader.rs   # Обработка файлов с Яндекс.Диска и локальных папок
│   ├── profile.rs      # Управление профилями
│   ├── uploader.rs     # Интеграция с API Wildberries
│   ├── utils.rs        # Вспомогательные функции
│   └── main.rs         # Точка входа приложения
├── Cargo.toml         # Зависимости и метаданные проекта
└── README.md          # Документация проекта
```

**English**  
```
├── src
│   ├── app.rs          # Main application logic and UI
│   ├── config.rs       # Configuration handling
│   ├── downloader.rs   # Yandex Disk and local file processing
│   ├── profile.rs      # Profile management
│   ├── uploader.rs     # Wildberries API integration
│   ├── utils.rs        # Utility functions
│   ├── main.rs         # Application entry point
├── Cargo.toml         # Project dependencies and metadata
└── README.md           # Project documentation
```

---

## Зависимости / Dependencies

**Русский**  
- `eframe` и `egui`: Для кроссплатформенного интерфейса.  
- `reqwest`: Для HTTP-запросов к API Яндекс.Диска и Wildberries.  
- `serde`: Для сериализации/десериализации JSON.  
- `arboard`: Для работы с буфером обмена.  
- `rfd`: Для нативных диалогов выбора файлов.  
- `anyhow`: Для обработки ошибок.  
- `log` и `env_logger`: Для логирования.  
- `regex`: Для разбора имен файлов.  
- `urlencoding`: Для кодирования параметров запросов.  
- `walkdir`: Для обхода локальных директорий.

**English**  
- `eframe` and `egui`: For the cross-platform GUI.  
- `reqwest`: For HTTP requests to Yandex Disk and Wildberries APIs.  
- `serde`: For JSON serialization/deserialization.  
- `arboard`: For clipboard functionality.  
- `rfd`: For native file dialogs.  
- `anyhow`: For error handling.  
- `log` and `env_logger`: For logging.  
- `regex`: For parsing file names.  
- `urlencoding`: For encoding API query parameters.  
- `walkdir`: For local directory traversal.

---

## Вклад в проект / Contributing

**Русский**  
Приветствуются любые вклады! Пожалуйста, следуйте этим шагам:  
1. Форкните репозиторий.  
2. Создайте новую ветку: `git checkout -b feature/your-feature`.  
3. Зафиксируйте изменения: `git commit -m "Добавление вашей функции"`.  
4. Отправьте в ваш форк: `git push origin feature/your-feature`.  
5. Откройте запрос на включение (Pull Request) с подробным описанием изменений.

### Стиль кода / Code Style  
- Следуйте стандартам Rust (`cargo fmt`).
- Добавляйте понятные логи для отладки и аудита.

**English**  
Contributions are welcome! Please follow these steps:  
1. Fork the repository.  
2. Create a new branch: `git checkout -b feature/your-feature`.  
3. Commit your changes: `git commit -m "Add your feature"`.  
4. Push to your fork: `git push origin feature/your-feature`.  
5. Open a pull request with a clear description of your changes.

### Code Style  
- Follow Rust’s standard conventions (`cargo fmt`).
- Include clear logging for debugging and auditing.

---

## Лицензия / License

**Русский**  
Проект распространяется под лицензией MIT. Подробности см. в файле [LICENSE](LICENSE.md).

**English**  
This project is licensed under the MIT License. See the [LICENSE](LICENSE.md) file for details.

---

## Благодарности / Acknowledgments

**Русский**  
- Спасибо сообществу Rust за отличные библиотеки, такие как `eframe` и `reqwest`.  
- Проект вдохновлён потребностью продавцов Wildberries в автоматизации загрузки контента.

**English**  
- Thanks to the Rust community for providing robust libraries like `eframe` and `reqwest`.  
- Inspired by the need for efficient content automation for Wildberries sellers.

---

## Контакты / Contact

**Русский**  
Для вопросов или предложений, пожалуйста, [откройте issue](https://github.com/your-username/wildberries-content-manager/issues).

**English**  
For issues or inquiries, please [open an issue](https://github.com/your-username/wildberries-content-manager/issues).

---

**Русский**: Разработано с 🦀 Rust для Wildberries 🚀  
**English**: Built with 🦀 Rust for Wildberries 🚀