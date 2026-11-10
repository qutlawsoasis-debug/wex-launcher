# Руководство по сборке релизов и настройке обновлений Wex Launcher

В этом документе описывается процесс ручной сборки установщиков Wex Launcher для Windows (NSIS и MSI), генерация манифеста автообновления `latest.json` и процедура настройки криптографической подписи.

---

## 1. Обзор процесса сборки

Сборка релизных пакетов автоматизирована через GitHub Actions workflow:
- **Файл workflow:** `.github/workflows/release-build.yml`
- **Триггер:** `workflow_dispatch` (запуск исключительно вручную пользователем)
- **Целевая платформа:** `windows-latest` (Windows x64)
- **Результаты сборки:**
  - **NSIS Installer:** исполняемый установщик `Wex_<version>_x64-setup.exe`
  - **MSI Package:** системный пакет развёртывания `Wex_<version>_x64_en-US.msi`
  - **Updater Manifest:** файл `latest.json` для встроенного механизма обновлений Tauri 2
  - **Артефакты:** все файлы выкладываются как **артефакты запуска (workflow artifacts)** и **НЕ публикуются автоматически в GitHub Releases**.

---

## 2. Настройка ключа подписи и автообновления (Tauri Updater)

Для безопасного распространения обновлений через плагин `tauri-plugin-updater` используется асимметричное шифрование (минисигнатуры Ed25519).

> [!IMPORTANT]
> **Ключ подписи категорически запрещено коммитить в репозиторий Git.** Приватный ключ хранится строго на локальной машине разработчика и в зашифрованных секретах GitHub Actions.

### Шаг 1. Генерация ключевой пары

На локальном компьютере выполните команду:
```bash
npx @tauri-apps/cli signer generate -w ~/.tauri/wex.key
```
Команда запросит пароль для защиты ключа (passphrase) и создаст два файла:
1. `~/.tauri/wex.key` — **приватный ключ** (держите в строгой тайне).
2. `~/.tauri/wex.key.pub` — **публичный ключ** (безопасен для открытого размещения).

### Шаг 2. Добавление публичного ключа в проект

Откройте файл открытого ключа `~/.tauri/wex.key.pub` и скопируйте его содержимое в конфигурацию `app/src-tauri/tauri.conf.json`:
```json
{
  "plugins": {
    "updater": {
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk..."
    }
  }
}
```

### Шаг 3. Добавление приватного ключа в GitHub Secrets

1. Перейдите в ваш репозиторий на GitHub: `https://github.com/qutlawsoasis-debug/wex-launcher`.
2. Откройте вкладку **Settings** -> в меню слева выберите **Secrets and variables** -> **Actions**.
3. Нажмите кнопку **New repository secret**.
4. Создайте секреты:
   - **Имя секрета:** `TAURI_SIGNING_PRIVATE_KEY`
     - **Значение:** скопируйте полное содержимое файла `wex.key` (или путь к ключу / ключ в формате base64).
   - **Имя секрета (если задавали пароль):** `TAURI_SIGNING_KEY_PASSWORD`
     - **Значение:** парольная фраза, которую вы указали при генерации ключа.

---

## 3. Поведение сборщика при отсутствии секрета

Если секрет `TAURI_SIGNING_PRIVATE_KEY` **не добавлен** в репозиторий:
- Сборка **не падает** с ошибкой.
- Workflow выводит предупреждение в логе:
  ```
  ::warning::TAURI_SIGNING_PRIVATE_KEY secret is not configured in repository secrets. Proceeding with unsigned build.
  ```
- Формируются полноценные неподписанные установщики NSIS (`.exe`) и MSI (`.msi`).
- Скрипт `scripts/generate-latest-json.ps1` генерирует манифест `latest.json` с пустым полем подписи `signature: ""`.

---

## 4. Манифест автообновления (`latest.json`)

Скрипт `scripts/generate-latest-json.ps1` автоматически сканирует созданные установщики и формирует манифест по спецификации `tauri-plugin-updater`:

```json
{
  "version": "0.9.1",
  "notes": "Wex Launcher manual release build",
  "pub_date": "2026-11-09T06:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<Ed25519 подпись из .sig файла>",
      "url": "https://github.com/qutlawsoasis-debug/wex-launcher/releases/download/v0.9.1/Wex_0.9.1_x64-setup.nsis.zip"
    }
  }
}
```

Скрипт поддерживает следующие параметры:
- `-Version`: версия приложения (по умолчанию считывается из `app/src-tauri/tauri.conf.json`).
- `-ArtifactsDir`: каталог с собранными бинарниками (по умолчанию `target/release/bundle`).
- `-Notes`: описание релиза для всплывающего окна обновления.
- `-BaseUrl`: базовый URL загрузки бинарников.
- `-OutputFile`: путь к итоговому файлу (по умолчанию `latest.json`).

---

## 5. Как запустить сборку релиза

1. Откройте GitHub репозиторий -> вкладка **Actions**.
2. В левой колонке выберите воркфлоу **Release Build**.
3. Нажмите кнопку **Run workflow**.
4. (Опционально) Укажите примечания к релизу в поле `Release notes`.
5. Нажмите зелёную кнопку **Run workflow**.
6. По завершении выполнения скачайте артефакты со страницы прогона:
   - `Wex-NSIS-Setup`
   - `Wex-MSI-Package`
   - `Wex-Updater-Manifest`
