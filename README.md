# YPractikum_Mod1

Проект для парсинга, конвертации и сравнения банковских выписок в форматах YPBank.

## Что делает
- Читает выписки в форматах `txt`, `csv`, `bin`.
- Конвертирует выписки между форматами.
- Сравнивает две выписки (содержимое операций; `meta` и `description` не участвуют в сравнении).

## Состав проекта
- Библиотека `utils`:
  - доменная модель (`Operation`, `Statement`) и логика сравнения/сортировки;
  - ошибки и диагностика;
  - парсеры/сериализаторы YPBank (csv/txt/bin).
- CLI-утилиты:
  - `converter` — конвертация форматов;
  - `comparer` — сравнение двух файлов.

## Установка и сборка
```bash
cargo build
```

## Использование

### Конвертер
```bash
cargo run --bin converter -- --in txt --out csv -i input.txt -o output.csv
cargo run --bin converter -- --in csv --out bin -i input.csv -o output.ypb
cargo run --bin converter -- --in bin --out txt -i input.ypb -o output.txt
```

Если `-i` не указан — чтение из stdin. Если `-o` не указан — запись в stdout.

### Сравниватель
```bash
cargo run --bin comparer -- --file1 a.bin --format1 bin --file2 b.csv --format2 csv
```

Вывод:
- `The transaction records are identical.` — выписки совпадают;
- `The transaction records are different.` — выписки отличаются.

## Тесты
```bash
cargo test
```
