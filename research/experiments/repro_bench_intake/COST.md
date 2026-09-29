# REPRO-Bench: стоимость обычного повторного запуска (2026-09-29)

Область: только целевые результаты [замороженного intake](RESULT.md), ID 51 и
109; лимит **20 МБ на файл**, а не на весь репликационный пакет. Оценки ниже —
время человека при уже доступных данных и ПО, **не замеры**. Скрипты авторов не
запускались; наборы 2,8 и 10,8 ГБ не скачивались. На этой машине нет `Rscript`
или Stata в `PATH`.

| Случай и целевой результат | ПО для обычного запуска; свободная альтернатива | Оценка времени | Лицензия данных и условия | Вердикт |
| --- | --- | --- | --- | --- |
| **51**, AJPS Table 2, Young RD: −0.084; [данные 494 704 Б](https://huggingface.co/datasets/chuxuan/REPRO-Bench/blob/74958fba32daeacff51f2e1fd37be3916595648f/51/replication_package/brazil-RD-data.dta) | Авторский `brazil-RD-analysis.R`: бесплатный **R 4.0.3** + `rdrobust`, `readstata13`, `Hmisc`. Stata-MP 16.1 указан в README для других таблиц, **не нужен для Table 2**. Python [`rdrobust`](https://github.com/rdpackages/rdrobust) возможен, но это отдельный перенос, не обычный повторный запуск. | **1–3 ч** на R, пакеты, пути и сверку таблицы; расчёт предположительно минуты, не измерен. Полный R-скрипт позднее читает ещё `brazil-validation-data.dta` (667 095 Б); целевой блок Table 2 — нет. | Исходный [Dataverse DOI](https://doi.org/10.7910/DVN/AWSQTW): [DataCite v1.1](https://api.datacite.org/dois/10.7910/dvn/awsqtw) указывает **custom terms** и open access, но не CC0. [Текст условий](https://dataverse.harvard.edu/api/datasets/:persistentId/versions/1.1/customlicense?persistentId=doi:10.7910/DVN/AWSQTW) недоступен (HTTP 403); пределы повторного использования и распространения **не установлены**. | **Нет сейчас**: сначала получить и прочесть custom terms; [I4R DP54](https://www.rwi-essen.de/fileadmin/user_upload/RWI/Publikationen/I4R_Discussion_Paper_Series/054_I4R_Kelly_Odermatt_Metson.pdf) уже независимо получил −0.084. |
| **109**, QJE Table III, Panel A, col. 3: 0.0191; [основные данные 19 735 542 Б](https://huggingface.co/datasets/chuxuan/REPRO-Bench/blob/74958fba32daeacff51f2e1fd37be3916595648f/109/replication_package/Data%20Files/county_gb_main.dta) | Авторский `CJLR_GreenBooks_QJE_Rep.do`: **лицензированный Stata**, SSC-пакеты из README/скрипта и дополнительно [`estout` (`eststo`/`esttab`)](https://github.com/benjann/estout), не объявленный в списке пакетов. R `fixest` или Python `statsmodels` позволяют перенести OLS с кластеризацией, но требуют проверки преобразований, фиксированных эффектов и SE; готовой эквивалентной замены не установлено. | При имеющейся лицензии **2–4 ч** на Stata, зависимости, пути и целевой блок; расчёт предположительно минуты, не измерен. Без лицензии срок её получения неизвестен; перенос в R/Python ориентировочно **4–8 ч или более**, не обычный запуск. | Исходный [Dataverse DOI](https://doi.org/10.7910/DVN/NXFB5R): [DataCite v1.1](https://api.datacite.org/dois/10.7910/dvn/nxfb5r) указывает **CC0 1.0** для набора данных. Это не лицензия статьи, стороннего отчёта или всей сборки HF. Лицензия на [Stata](https://www.stata.com/order/purchasing-faqs/) приобретается отдельно. | **Нет сейчас**: Stata здесь отсутствует; [I4R DP140](https://www.econstor.eu/bitstream/10419/301429/1/I4R-DP140.pdf) уже получил **0.0263**, отличающееся от статьи. Новый запуск нужен лишь при новом, заранее сформулированном вопросе. |

Основание для оценки ПО и зависимостей: авторские README и скрипты в
[зафиксированной версии REPRO-Bench](https://huggingface.co/datasets/chuxuan/REPRO-Bench/tree/74958fba32daeacff51f2e1fd37be3916595648f).
Для 109 целевой коэффициент использует только `county_gb_main.dta`, но **весь**
блок Table III дополнительно читает два файла на ~3,0 и ~3,7 МБ, поэтому его
суммарный объём превышает 20 МБ. Равенство содержимого HF-копии и исходных
Dataverse-наборов отдельно не проверялось; лицензионные выводы относятся к
записям исходных DOI. По существующему отчёту и без нового проверяемого вопроса
обычный просмотр источников дешевле обоих повторных запусков. Этот обзор не
устанавливает спрос на новый верификатор или преимущество над таким просмотром.
