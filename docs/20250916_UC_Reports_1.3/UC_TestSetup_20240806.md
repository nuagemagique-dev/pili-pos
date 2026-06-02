# UC_TestSetup_20240806

> 1:1 text extraction of [`UC_TestSetup_20240806.pdf`](<../original_pdfs/20250916_UC_Reports_1.3/UC_TestSetup_20240806.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved — refer to the source PDF for those. Text content is verbatim.

```text
THE FPS FINANCES
EXPERIENCE
Boulevard Albert II 33

1030 Brussels



Vat-number: BE0000000097, establishment number: 2000000042



This restaurant has three diﬀerent dining areas:

    -   the BAR for drinks and tapas (table 1 to 4 with standard 4 chairs and table bar with 8 high
        chairs);
    -   the DINING ROOM for dining (table 5 to 10 with 4 chairs, tables 11 to 15 with 2 chairs);
    -   the TERRACE for lunch and dinner (tables 16 to 25 with 2 chairs).

Furthermore there are 4 hotel suites available for our guests.

The restaurant is equipped with a POS system composed by two POS, serial numbers CPOS0031234567
and CPOS0031234568, resp. linked to the FDM FOD01000001 and FOD01000002. Both FDM are
available for load balancing. POS xx567 is physically placed on the BAR, POS xx568 is located at the
recepfion desk and also contains our hotel PMS. The POS software is linked through the PMS API.

The restaurant has in each of the dining areas a ﬁxed tablet acfing as a terminal connected to POS
xx567. In the DINING ROOM and on the TERRACE 4 handhelds (a1b2c3d4, e5f6g7h8, i9j1k2l3,
m4n5o6p7) can be used.

The restaurant has a ﬁxed payment terminal at the BAR, connected to the POS in the BAR (EFT). There’s
also a mobile payment terminal, to be used in all dining areas.

The staﬀ: John (bartender, employeeId 75061189702), Elisa (chef de salle, employeeId 84022899837)
and Sammy (sommelier, employeeId 80113078968). Other staﬀ are not allowed to register orders/sales
on the POS.
          The limited menu:

                                                                            unitPrice
productId             productName         departmentId   departmentName    (VAT incl.)   unit    label               info
 10006      Dry Martini                       10         Aperitifs              12,00    PIECE       A
 12007      Perrier                           12         Soft Drinks              2,50   PIECE       A
 12009      Coca Cola                         12         Soft Drinks              3,04   PIECE       A
 15001      Bourgogne Red                     15         Wines                  35,00    PIECE       A
 15002      Chardonnay White                  15         Wines                  40,00    PIECE       A
 15003      Wine (glass)                      15         Wines                    8,00   PIECE       A
 15023      Matching Wines                    15         Wines                  24,00    PIECE       A
 20001      Carpaccio Beef                    20         Starters               20,00    PIECE       B
 22001      Burger of the CHEF                22         Main Dishes            28,00    PIECE       B
 22005      Catch of the day                  22         Main Dishes            45,00       KG       B
 22010      Dish of the day                   22         Main Dishes       open price    PIECE       B
 25003      Dame Blanche                      25         Desserts               11,52    PIECE       B
 25008      crème brûlée                      25         Desserts               10,00    PIECE       B
 28007      Tapas variation                   28         Tapas                    3,34   PIECE       B
 30001      Business menu (*)                 30         Menus                  38,00    PIECE       B    (*) = 1 Carpaccio + 1 Burger

                                                                                                         (**) = 1 Carpaccio + 1 Burger
 30010      Business menu all-in (**)         30         Menus                  58,00    PIECE   A&B               + 1 matching wines
                                                                                                             (***) = 1 aperitif + 1 soft
 30020      Menu drink (***)                  30         Menus                  12,00    PIECE       A                            drink
 40001      Gift Voucher General              400        Vouchers               50,00    PIECE       X
 40002      Gift Voucher General              400        Vouchers              100,00    PIECE       X
 40003      Gift Voucher General              400        Vouchers              150,00    PIECE       X
 40004      Gift Voucher General              400        Vouchers              250,00    PIECE       X


 40020      Voucher Special Diner Offer       400        Vouchers               75,00    PIECE       B
 90001      Bottle S                          90         Bottle Deposits          0,01   PIECE       X
 90100      Beer bin/ keg beer                90         Bottle Deposits          4,49   PIECE       X
 500001     Cigarettes (20 pcs)               500        Cigarettes               8,40   PIECE       D




          Guests can move from one dining area to another (e.g. starfing with a dry Marfini in the bar, having
          diner in the dining room and closing the evening with glass of brandy on the terrace).

          The restaurant is very ﬂexible and fully customer orientated; changes on tables and/or chairs,
          discounts, vouchers, fips, foreign currency, copy receipt, invoices …

          The following payment methods are accepted: Bancontact, Visa debit, Mastercard debit, Visa credit,
          Mastercard credit, Diners Club, American Express, Pluxee, Edenred, cash (EUR and GBP/USD), gift
          voucher, Payconiq, Apple Pay.
Summarized:
Physical Devices:

POS
 Type                POS ID                Terminal ID            Device ID
 Computer            CPOS0031234567        POS-1-BAR              F0:1D:3D:CC:13:C8
 Computer            CPOS0031234568        POS-2-REC              80:1A:A6:75:66:FC


FDM

 FDM ID              Accepts requests from
 FOD01000001         CPOS0031234567        CPOS0031234568
 FOD01000002         CPOS0031234567        CPOS0031234568


Terminal


 Type                Terminal ID               Device ID
 Tablet              TER-1-BAR                 b54a614f-39cc-4a7b-bd9f-aa6b693d769c
 Tablet              TER-2-DIN                 1631678d-7a85-4ac3-b296-bb4565e873fe
 Tablet              TER-3-TER                 58d5ce57-e32e-4463-b3fe-b43513089669
 Handheld            TER-4-HH                  a1b2c3d4
 Handheld (backup)   TER-4-HH                  e5f6g7h8
 Handheld            TER-5-HH                  i9j1k2l3
 Handheld (backup)   TER-5-HH                  m4n5o6p7


Drawer

 ID                         Name                           Connected to
 1                          Drawer-1                       CPOS0031234567


Gift Vouchers:

 Type                       Name                           Price
 MPV                        Gift voucher general           50-100-150-250 euro
 SPV                        Special Diner oﬀer (excl.      75 euro
                            Beverage)

```
