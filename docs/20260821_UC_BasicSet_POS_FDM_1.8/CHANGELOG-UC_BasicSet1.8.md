# CHANGELOG-UC_BasicSet1.8

> 1:1 text extraction of [`CHANGELOG-UC_BasicSet1.8.docx`](<../original_pdfs/20260821_UC_BasicSet_POS_FDM_1.8/CHANGELOG-UC_BasicSet1.8.docx>) (via `textutil`). Text content is verbatim.

```text
CHANGELOG Use Cases – Basic Set

Version
Date
Summary
1.0
08/08/2024
Publication Basic Set
1.1
06/09/2024
Corrections: 
	•	M110: remove trailing space in posId
	•	M121: remove trailing space in terminalId
	•	M171: ticketMedium: PAPER instead of NONE
1.2
24/09/2024
All use cases:
	•	All queries: fields warnings and informations updated to the latest version of type system:  { message locations { line column } extensions { category code data { name value } showPos }
1.3
07/10/2024
M112: Partial Refund: 
	•	Original sale added
	•	Ordered products adapted
1.4
23/12/2024
	•	M161 Corrected scenario (copy of order)
	•	All UC with Cash payment: drawer was added to the financial line
1.5
24/01/2025
M112: split in 2 UC’s:
	•	M112A: cash register supports partial refund in combination with new sale
	•	M112B: cash register doesn’t support partial refund in combination with new sale
	•	M112A_2: CASH payment 0.01 is ROUNDING (not TIP)
1.6
23/05/2025
M171: correction in response FDM (no vatCalc)
1.7
16/09/2025
M171B: example alternative query with directives Status query: examples Q100, Q110
Footer: field ‘footer’ added in signSale queries
1.8
21/08/2026
	•	M121: response: remove trailing space in terminalId

```
