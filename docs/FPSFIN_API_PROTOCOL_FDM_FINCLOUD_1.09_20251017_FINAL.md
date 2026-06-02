# FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09_20251017_FINAL

> 1:1 text extraction of [`FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09_20251017_FINAL.pdf`](<original_pdfs/FPSFIN_API_PROTOCOL_FDM_FINCLOUD_1.09_20251017_FINAL.pdf>) (via `pdftotext -layout`). Diagrams, images, and exact PDF visual layout are not preserved — refer to the source PDF for those. Text content is verbatim.

```text
                           MD 29/04/2024
REGISTERED CASH REGISTER SYSTEM –
     TECHNICAL SPECIFICATIONS


                 API-PROTOCOL BETWEEN
                           FDM & FINCLOUD



                           GKS     SCE

VERSIE 1.09 – 17/10/2025
GKS2 Protocol v1.09 TBP

 1. Version history
 2. Preamble
 3. Context
 4. Actors
 5. Data elements in the Fiscal Data Module (FDM)
 6. FDM certificate management
     6.1 FDM device identity
     6.2 Claim certificate(s)
     6.3 Authentication
     6.4 Trust certificates
 7. System clock
 8. Booting the FDM
     8.1 Initialization boot
     8.2 Regular boot
 9. FDM Receiving & sending business transactions
     9.1 FDM buffer
     9.2 Sending business transactions
         9.2.1 Postpone transmission
         9.2.2 Maximum hourly transmission speed
         9.2.3 Transmission regulation slots
 10. FDM Service interruption
     10.1 FDM-POS mismatch
     10.2 FDM buffer limit
     10.3 FDM lock
 11. Administration console
 12. Webservices
     12.1 General
     12.2 Error logging
     12.3 Initialization webservice
         12.3.1 Reply with retry invitation
         12.3.2 Successful reply
         12.3.3 Exceptions
     12.4 Register webservice
         12.4.1 Register business transactions
         12.4.2 No Operation message
         12.4.3 Replies
     12.5 Query webservice
         12.5.1 Get the queries to-do list
         12.5.2 Queries root structure
         12.5.3 FDM reply structure
         12.5.4 Replies executed queries
         12.5.5 Queries description
             12.5.5.1 Mutation: SetFdmLock
             12.5.5.2 Mutation: SetParams
                                                    1
           12.5.5.3 Mutation: SetVatRates
           12.5.5.4 Mutation: SetPosAllowlist
           12.5.5.5 Mutation: SetTrustCert
           12.5.5.6 Mutation: SetUrls
           12.5.5.7 Mutation: SetClientCert
           12.5.5.8 Query: FailedRequests
           12.5.5.9 Query: AllFdmInfos
  13. Warnings and errors summary
  14. Canonical JSON for repeatable hashes




1. Version history
Date        Versio       Change
            n
05/08/ 1.0               Initial version
2024
05/09/ 1.01              Improvements based on feedback
2024
12/09/2 1.02             ws_init. clarifications & cleaning up the text
024
04/10/      1.03         minor corrections
2024
12/02/2 1.04             text improvements, minor corrections in section 12.5
025
22/02/ 1.05              12.3.3. - adding solution avoiding loops (based on feedback)
2025
14/03/      1.06         retryIntervalInit added in sections 5,11 and 12.3.3
2025
12/05/2 1.07             correction url: https://www.gs2.be / timeStamp
025
13/08/2 1.08             Major important improvements, as a result of live testing:
025                      1. adding the following data elements: vatNo, estNo and protocolVersion =>
                            changes in 5, 9.1, 9.2, 12.3.2, 12.4.1, 12.4.2, 12.5.2, 12.5.3, 12.5.5.2, 12.5.5.9
                            and 13
                                                          2
                    2. 12.1: important note on size messages

                    3. 12.3.2 & 12.5.5.7: minor change concerning ‘shared secret’.

17/10/2 1.09          Introduction ‘Emergency FDM’: changes in 4. actors, 9.2 Sending business
025                   transactions, 12.4.1. Register business transactions and 12.5.5.4. Mutation:
                      SetPosAllowList
                      Specifying ntp and ntp2 in Data Elements
                      Adding table of content


2. Preamble
This document explains the high-level design, core technicalities and main functionalities of the
protocol between the FDM and FPS Finance.
Intended audience of this document: manufacturers interested in building an FDM for the
Belgian market.



3. Context
To increase tax compliance, all business transactions recorded by the use of mandatory RCRS
(currently limited to establishments offering restaurant and catering services that exceed the
limit of 25.000 euro sales turnover) should be sent electronically to FPS Finance for
analysis/horizontal monitoring. FPS Finance can use this information source for more targeted
tax audits.



4. Actors
  FPS Finance
    main stakeholder and organizer of the project
    provides specifications for POS & FDM (but doesn’t produce them)
    certifies POS & FDM
    provides infrastructure to receive business transactions electronically
    provides infrastructure to manage authentication: creating & renewing client certificates
  Manufacturer
    produces POS / FDM following specifications of FPS Finance

                                                 3
    attributes a serial number and secret hardware key to the FDM, and dispatches this
    information to FPS Finance following an exchange protocol
    gets their products certified by FPS Finance
    provides support for their produced devices
  Businesses
    registers its POS and FDM devices with FPS Finance
    uses POS and FDM following legal instructions
    buys his POS and FDM from the Manufacturer (or one of its dealers)
    asks support for POS / FDM from the Manufacturer (or one of its dealers)
  FDM (= black box)
    computing device permanently connected to the POS and requiring frequent internet
    access availability
    electronically signs all incoming business transactions
    sends all business transactions to Cloud Partner
  Emergency FDM
    Replacement FDM
    Use in case of a FDM breakdown
    After settings, can start directly when plugging in (no need to call ws query before use in a
    businesses).
    Used during a short time
    Use estNo & vatNo from POS instead of SetPartams mutation values to register
    business transactions
    Allow all POS
  POS (= cash register)
    manages and stores all orders and business transactions
    manages and displays messages from FDM
    sends all business transactions to FDM



5. Data elements in the Fiscal Data Module (FDM)
These data elements should be maintained by the FDM. Write access “ws” means that the value
can only be updated with information coming from FPS Finance webservices. Write access
“fdm” means that only the FDM can update the value.
                                                4
Nr   Abbrevia Explanation        Data type     Read Write Factory           Special
     tion                                      access access default        security
                                               by            value
                                               user
1    fdmId     FDM             11            yes      nobod   provided by Can
               identification  alphanumeri            y       manufacture never be
               number, used to cal positions                  r           modified
               uniquely                                                   .
               identify the
               FDM unit
2    fdmSwVe firmware       text               yes    fdm
     rsion   version of FDM
3    fdmHkey   hardware key, private key       no     nobod   factory       Stored in
               used to verify   256 bits              y       generated     security
               the identity and                               private key   enclave
               to sign business                               using         (private
               transactions                                   secp256r1     key).
                                                              prime field   Created
                                                              curve         for
                                                                            lifetime
                                                                            of the
                                                                            device.
                                                                            Private
                                                                            key may
                                                                            never be
                                                                            exposed
                                                                            in
                                                                            logging.
4    fdmLock   FDM lock.         boolean       yes    ws      false
               If set to true,
               the device
               becomes
               locked and will
               no longer

                                           5
               accept or sign
               any business
               transactions.
5   lockReas   lock reason for   array of    yes     ws    FALSE
    on         fdmLock           language
                                 and
                                 message for
                                 lock reason
6   posIdList list of allowed array of         yes   ws
              PosId numbers strings
              by the FDM
               list containing
               data elements
               of 14
               alphanumerical
               positions
7   fdmLastA date of last      timestamp       yes   fdm   null (only for
    ctions   retrieval of                                  first boot
             queries task list                             ever)
             (ws_query or
             ws_init)
8   fdmLastS timestamp of        timestamp     yes   fdm
    uccessful last successful
    Trans     call to register
              webservice (in
              UTC)
              ( ws_reg or
                ws_nop )
9   fdmInit    If set to true,   boolean       no    fdm
               the device
               should try to
               perform an init
               during boot.


                                           6
10   initInterva During               integer        no   ws
     l           unsuccessful
                 calls to
                 ws_query,
                 interval in
                 seconds at
                 which the FDM
                 should call
                 ws_init.
11             interval in     integer               no   ws
     queryInte seconds at
     rval      which the
               FDM should call
               ws_query
12   nopInterv interval in            integer        no   ws
     al        seconds at
               which the
               FDM has to
               report
               “No Operation”
13   retryInter retry intervals       Array of       no   ws   [5, 60, 300,   Omit
     vals       expressed in          integers                 300, 300,      from
                seconds                                        3600,          logging.
                 If the list of                                10800]
                 retry intervals is
                 exhausted due
                 to persistent
                 failed calls, the
                 last value may
                 be repeated.
                 Use for
                 ws_reg and
                 ws_query


                                                 7
14   retryInter retry intervals       Array of       no    ws    [5, 60, 300,   Omit
     valsInit   expressed in          integers                   300, 300,      from
                seconds                                          3600,          logging.
                 If the list of                                  10800]
                 retry intervals is
                 exhausted due
                 to persistent
                 failed calls, the
                 last value may
                 be repeated.
                 Use for
                 ws_init
15   retryCoun number of              integer        yes   fdm
     t         retries of sent
               attempt
16   vatFromB VatRatesI array of         yes               ws                   The FDM
     ookingDat nput array of VatRatesInp                                        should
     eList                   ut                                                 apply the
                 vatRates
                                                                                correct
                 array and
                                                                                VAT
                 fromBooki                                                      rates
                 ngDate                                                         accordin
                 Each                                                           g to the
                                                                                date of
                 VatRatesI
                                                                                use.
                 nput
                 element looks
                 as follows: see
                 12.5.5.3


                    vatLabe
                    l contains
                   the VAT label
                   A, B, C, D or
                   X,
                                                 8
                  outOfSco
                  pe
                 (boolean) no
                 VAT applied,
                 only for
                  vatLabe
                  l X
                 (vatRate = 0
                 when true)
                  vatRate
                 contains the
                 VAT rate in
                 decimal value

17   footer    receipt footer    array of       yes   ws
               Strings may not strings
               contain new
               lines or carriage
               returns. Array
               item is
               considered as
               single
               paragraph of
               the footer.
18   url       url to print on   string         yes   ws
               receipt footer
19   unsentEv current number integer            no    fdm   0   Omit
     entCount of unsent                                         from
              events                                            logging.
              transactions in
              the buffer
20   postpone If set to true, boolean           no    ws        Omit
              the FDM should                                    from
              postpone                                          logging.
              sending
                                            9
                business
                transactions till
                 maxMsgSiz
                 e is reached.
21   maxMsgS The maximum integer                yes   ws   32 KB   Omit
     ize     size in KB of                                         from
             register                                              logging.
             webservice
             message body.
             This value will
             determine how
             many business
             transactions
             can be sent in a
             single
             webservice call.
             If the size of
             one business
             transaction >=
                 maxMsgSiz
                 e , this
                transaction can
                be sent despite
                 maxMsgSiz
                 e . The
                message body
                contains only
                this transaction.
22   defaultMa maximum           integer        no    ws   0       Omit
     xCalls    number of                                           from
               webservice                                          logging.
               calls to register
               webservice per
               hour


                                           10
23   maxCalls   list of regulation array of        no    ws               Omit
                slots to limit the regulation                             from
                calls to the       slots                                  logging.
                register
                webservice
                A regulation slot
                is an object with
                3 properties:
                day of week,
                hour slot and
                maximum
                number of calls.
24   maxBuffe the maximum           integer        no    ws    0          Omit
     r        limit for unsent                                            from
              business                                                    logging.
              transactions in
              the buffer
25   paramsVe Date and time timestamp              yes   fdm   empty      Omit
     rsion    of the params                                               from
              version                                                     logging.
              currently used
              by the FDM. A
              params version
              is an array of
                ParamsFro
                mSystemDa
                teInput (
                fromSyste
                mDate and
                params ).
26   ws_reg     webservice to       hostname       yes   ws    will be
                register            and portnr                 provided
                business
                transaction(s)
                                              11
27   ws_nop      webservice to       hostname     yes   ws     will be
                 post NOP            and portnr                provided
                 transaction
28   ws_init     webservice to       hostname     yes   user   will be
                 (re)initialize an   and portnr         and ws provided
                 FDM
29   ws_query webservice to          hostname     yes   ws     will be
              check if there         and portnr                provided
              are queries to
              do
30   ws_ntp1,    NTP time       hostname          yes   ws &   ntp1 =
     ws_ntp2,    server. We use and portnr              user   time.aws.co
     ws_ntp3     UTC time zone.                                m, ntp2 =
                 The FDM                                       time.belnet.
                 should use in                                 be
                 priority ws_ntp1
                 and ws_ntp2. If
                 they don’t work,
                 the FDM can
                 use the
                 ws_ntp3.
                 ws_ntp3 can
                 be specified
                 only manually
                 via the
                 administration
                 console.
31   certstoreT certificate store set of X.509 yes      ws                    Omit
     rust       containing trust certificates                                 from
                certificates to   (PEM)                                       logging.
                resolve server
                certificate
32   certstore   root and            set of X.509 yes   no     preconfigur    Omit
     Hw          intermediate        certificates              ed in          from
                                            12
                hardware           (PEM)                     firmware   logging.
                certificate
                published by
                FDM
                manufacturer
33   certClient client certificate PKCS #12     yes     ws              Stored at
                used in the        format       (except                 least in
                mTLS session                    the                     non-user
                This client                     private                 accessib
                certificate is                  key)                    le
                valid for one                                           memory.
                year (this value                                        Ideally
                can be                                                  stored in
                modified,                                               security
                adapted by FPS                                          enclave.
                Finance                                                 Private
                according to                                            key may
                business or                                             never be
                technical                                               exposed
                needs).                                                 in
                                                                        logging.
34   clientCert serial number of string         yes    ws
     Sn         the client
                certificate
35   certClaim claim certificate                                        Omit
               used in the                                              from
               initialization                                           logging.
               webservice
36   fdmDateTi RTC time of         timestamp
     me        FDM
                We assume
                here that an
                RTC that is not
                powered by
                main power or
                                           13
                      battery adopts
                      a default
                      date/time in the
                      past.
37        vatNo       VAT number         string        yes       ws
                      without 'BE'       (length=10)
38        estNo       Establishment      string        yes       ws
                      number             (length=10)
39        protocolV Version of the       string        yes       ws
          ersion    protocol that        (length=10)
                    must be used
                    by the FDM




6. FDM certificate management
6.1 FDM device identity
Each FDM has a unique identity which is provided by the FDM manufacturer. This identity
consists of an fdmId , hardware key fdmHkey and certificate for life (use an expiration
date in the very far future ). The hardware key fdmHkey is a secp256r1 prime field curve
private key and is stored in the security enclave of the FDM. The hardware certificate is a digital
certificate that should have the following elements:
  SHA-256 hashing algorithm
  Elliptic Curve Digital Signature (ECDSA) signing algorithm
  subject name of the certificate contains at least the fdmId of the FDM
  SHA-256 fingerprint
  certificate serial number with producer prefix (All hardware certificates issued by an FDM
  manufacturer should have a unique certificate serial number.)
  no expiration date (use an expiration date in the very far future)
The hardware key will also be used to sign business transactions.
A secure protocol will be established so that the manufacturer can dispatch the hardware
certificates to FPS Finance.
                                                 14
The FDM manufacturer is also responsible for establishing a root and one or more intermediate
certificates for the hardware certificate conforming industry best practices. The root and
intermediate certificates should never expire and should be part of the FDM firmware (a.k.a.
 certstoreHw ).




FPS Finance will establish a collaboration protocol with FDM manufacturers which includes
security requirements during production of the FDM units. In case of security issues, FPS
Finance reserves the right to permanently revoke access to the online services.

6.2 Claim certificate(s)
During the first boot ever of an FDM unit, or when its client certificate has expired, an mTLS
session can’t be established. In this case, FPS Finance adds an electronic signature to its reply.
The FDM should verify this signature using the claim certificate(s). To this end, the FDM
manufacturer creates a never-expiring intermediate certificate derived from its root certificate
and hands over the private key of this intermediate certificate to FPS Finance. The latter will be
used by FPS Finance to establish its claim certificates. This concept is illustrated in the following
diagram.




The usage of claim certificates is further explained in section 12.3.

6.3 Authentication
The main authentication and encryption mechanism of the FDM is based on mutual TLS in
which both a server and client certificate are involved. The FDM contains a client certificate
issued by FPS Finance. This client certificate is valid for one year (this value can be modified by

                                                 15
FPS Finance according to business or technical needs). The private key should be stored in a
security enclave of the FDM device. FPS Finance will generate the private key using secp256r1
prime field curve. The hashing and signing algorithms to use are SHA-256 and Elliptic Curve
Digital Signature (ECDSA). The subject name of the certificate will amongst others contain the
 fdmId of the FDM. The procedure to issue a new client certificate is described in section
12.3. An automatic update of an operational client certificate is described in section 12.7.
If the client certificate is valid for less than 4 months, the user should receive a warning
 CLIENT_CERT_NEAR_EXPIRATION appearing on the POS at the next possible
opportunity. If the client certificate has expired, the user should receive a warning
 CLIENT_CERT_EXPIRED appearing on the POS at the next possible opportunity. The
user can check the details of the client certificate via the Administration Console.
In case of abuse, FPS Finance may decide to retract the client certificate. The user can consult
the FPS Finance e-service to find out if his access has been retracted.

6.4 Trust certificates
FPS Finance will configure a server certificate for its webservices. This server certificate is
typically updated yearly. The FDM should maintain a certificate store certstoreTrust to
be able to resolve the server certificate. If the FDM can’t resolve the server certificate, the user
should receive a warning SERVER_CERT_RESOLVE_FAILED on the linked POS systems
at the next possible opportunity. The FDM is not allowed to unconditionally trust the server
certificate (except for the initialization webservice described in section 12.3).
FPS Finance will dispatch the trust certificates automatically. The FDM may delete expired
certificates from the certificate store certstoreTrust .



7. System clock
The system clock should be synchronized at each system startup or restart, and at least every
24 hours using ntp service. If the synchronization fails, a warning should be shown on the
POS systems at the next possible opportunity. This warning should be
 RTC_NOT_INITIALIZED when fdmDateTime <
fdmLastSuccessfulTrans , or RTC_SYNC_FAILED otherwise.
To synchronize the system clock, the FDM first tries ws_ntp1 and falls back to ws_ntp2 if need
be. If both fail, ws_ntp3 can be attempted. This latter is a local configured NTP service entered
                                                 16
manually via the administration console.
The FDM operates in the UTC time zone.



8. Booting the FDM
This section describes the minimal requirements of the FDM boot sequence. The first step
during boot is to check the expiration date of the client certificate. We distinguish 2 possibilities:
  The FDM boots in initialization mode (see section 8.1) in the following situations. For each
  situation, we set fdmInit to true .
     FDM has an expired or non-existent client certificate certClient .
     FDM has an expired Trust Certificate or non-existent certstoreTrust .
     If fdmLastActions is null.
     Or fdmInit was already set to true during a previous boot or other.
  Otherwise, the FDM boots in regular mode (see section 8.2).

    Before the first boot ever, the owner of the FDM should contact FPS Finance to register the
    FDM and the linked POS systems.

8.1 Initialization boot
The following actions should be performed in the order in which they appear. In the specific
case of an expired or nonexistent client certificate, all steps are required and should be
successful to get the FDM operational.
  The FDM should synchronize the system clock (see section 7).
  The FDM will call the initialization webservice ws_init and install the initialization
  package (see section 12.3). If this call fails persistently, the owner might have to verify, and
  update if needed, the URL of ws_init manually in the Administration Console. After
  successful execution of the initialization webservice, set fdmInit = false and update
  the fdmLastActions timestamp.
  If fdmDateTime > fdmLastSuccessfulTrans , the FDM verifies whether there
  are unsent business transactions. If so, these are sent in accordance to section 9.2 “Sending
  business transactions”. If there are no transactions to be sent, the FDM will send a NOP
  instead (section 12.4.2).

                                                  17
8.2 Regular boot
The following actions should be performed in the order in which they appear. None of the
actions are blocking.
  The FDM should try to resynchronize the system clock with the NTP time server (see section
  7).
  If fdmDateTime > fdmLastSuccessfulTrans , the FDM will:
    Verify whether the query webservice needs to be contacted (section 12.5).
    Verify whether there are unsent business transactions. If so, these are sent in accordance
    to section 9.2 “Sending business transactions”. If there are no transactions to be sent, the
    FDM will send a NOP instead (section 12.4.2).



9. FDM Receiving & sending business transactions
9.1 FDM buffer
All valid incoming business transactions from the POS are enriched and signed immediately.
If the FDM is locked ( fdmLock = true ), all incoming business transactions will be
refused and the error lockReason message should be shown on the linked POS systems at
the next possible opportunity (in accordance with the POS language choice).
If the values inside the business transactions from the POS for vatNo and/or estNo are
not the same as the values on FDM parameters, the warning UPDATE_POS_VATNO and/or
UPDATE_POS_ESTNO should be shown on the linked POS systems at next possible
opportunity.
In order to have a repeatable hash for the electronic signature, canonical JSON should be used
(see section 14 for details). The electronic signature consists of a SHA-256 hash, encrypted with
the fdmHkey using Elliptic Curve Digital Signature Algorithm (ECDSA) in base64 output. The
business transactions, their electronic signatures and localization information (GPS coordinates)
are then stored temporarily in the persistent memory of the FDM. We refer to this memory as the
buffer of the FDM. The FDM always tries to ventilate from the oldest business transaction first to
the most recent business transactions last. The FDM should maintain an
 fdmLastSuccessfulTrans timestamp indicating the last successful call to the register
webservice ( ws_reg or ws_nop ) and retryCount indicating the number of eventual

                                                18
call retries after failure. After each successful sending, fdmLastSuccessfulTrans is
updated and the retryCount is reset (see section 13.4).
The buffer should contain at least:
  the full business transaction
  the timestamp of successfully sending the transaction
The FDM keeps tracks of which transactions it still has to send. If a business transaction has
been sent successfully, it may be deleted after 10 days. If the FDM reaches its physical memory
limit, sent business transactions may be deleted exceptionally before 10 days, always starting
with the oldest transactions.
The buffer with unsent business transactions is limited in size, see section 10.2 for more details
and implications.
The buffer contents can be read by the user via the Administration Console (see section 11).

9.2 Sending business transactions
In the general case, all business transactions should be sent to FPS Finance without delay.
Additionally, the FDM should also send a “No Operation” (NOP) message in case the buffer is
empty and the current timestamp exceeds fdmLastSuccessfulTrans +
 nopInterval . In case the buffer remains empty, this NOP message should be repeated at
each nopInterval . The NOP message is a regular webservice call with an empty
transaction list. This is further explained in section 12.4 which explains the register webservice
( ws_reg ).
Remark : to send business transactions correctly, the FDM must have a value for fields estNo
and vatNo (not null, not empty). See 12.4.1 Register business transactions. If this is not the
case, business transactions remain in the buffer. This values are only set by the SetParams
mutation (see 12.5.5.2 Mutation: SetParams). An exception to this rule, the Emergency FDM,
uses the estNo & vatNo of the values inside the business transactions from the POS (no
need to call ws query before use in a businesses).
There are however some exceptions about when to call the register webservice ( ws_reg )
which are used for load balancing purposes at the client side. These are described in the
following subsections 9.2.1, 9.2.2 and 9.2.3.

    The FDM may never ignore the exceptions described in sections 9.2.1, 9.2.2 and 9.2.3,
    except for NOP messages. FPS Finance will reduce the risk of a full buffer, with amongst
                                                 19
    others the following measures:
       fine-tune the parameters defaultMaxCalls , maxCalls , maxMsgSize ,
       and maxBuffer
       require that the manufacturers install enough physical memory in the FDM devices, so
       that maxBuffer can be set sufficiently high

The FDM will only function with the POS systems it is registered for (see section 10.1).
9.2.1 Postpone transmission

If postpone = true , the FDM should postpone calls to the register webservice
( ws_reg ) till maxMsgSize is reached, so several business transactions can be bundled
in 1 webservice call. If, due to a very low number of business transactions, the level of
 maxMsgSize isn’t reached within 60 minutes, a call is performed anyway if there is no
regulation slot preventing it - see section 9.2.3.
When postpone = false , there is no restrictions on sending transactions
( defaultMaxCalls & maxCalls are not applied)
9.2.2 Maximum hourly transmission speed

If defaultMaxCalls > 0 , the FDM can call the register webservice maximally
defaultMaxCalls times per hour. Each call to the register webservice can contain
multiple business transactions, but is limited in physical size by maxMsgSize . The FDM
should try to evenly distribute the webservice calls if possible.
9.2.3 Transmission regulation slots
This mechanism allows to further refine the maximum hourly transmission speed explained in
the previous section. If the maxCalls array is not empty, the FDM should respect the
regulation slots.
A regulation slot contains 3 elements.
  The first element refers to the day of the week: Monday, Tuesday, Wednesday, Thursday,
  Friday, Saturday, and Sunday.
  The second elements refers to an hourly slot based on a 24 hour clock. For example, 18 refers
  to the hourly timeslot between 18:00 and 19:00 o’clock.
  The last element refers to the maximum number of allowed webservice calls to the Register
  webservice. If this value is 0, then no calls can be performed in that hourly timeslot.
                                                 20
GraphQL description for regulation slots:
The enum of the first element:
1 enum WEEKDAY {
2   MONDAY
3   TUESDAY
4   WEDNESDAY
5   THURSDAY
6   FRIDAY
7   SATURDAY
8   SUNDAY
9 }




Definition of MaxCallsInput :
1 input MaxCallsInput {
2   dayOfWeek: WEEKDAY!
3   hour: Integer!                # range 0 - 23.
4   maxCallsInSlot: Integer!      # The maximum number of calls allowed in the specified time slot.
5 }




Regulation slots sample:
The following example forbids the FDM to call the register webservice on Saturdays between
18:00 and 19:00 o’clock and on Mondays between 17:00 and 18:00 o’clock. Furthermore, on
Sundays, the maximum hourly transmission speed is 100 calls between 12:00 and 13:00 o’clock.
 1 [
 2   {
 3     "dayOfWeek": "SATURDAY",
 4     "hour": 18,
 5     "maxCallsInSlot": 0
 6   },
 7   {
 8     "dayOfWeek": "SUNDAY",
 9     "hour": 12,
10      "maxCallsInSlot": 100
11   },
12   {
13      "dayOfWeek": "MONDAY",
14      "hour": 17,
15      "maxCallsInSlot": 0
16   }
17 ]




                                                 21
10. FDM Service interruption
10.1 FDM-POS mismatch
The FDM only accepts business transactions from the POS devices it is registered for. The
check is done via the identification number of the POS: posIdList . This value is
communicated by FPS Finance via the SetPosAllowlist query of the query webservice
( ws_query ). See section 12.5.5.4.
When the FDM is connected to a wrong POS system, the user should receive an error
UNKOWN_POS via the linked POS systems at the next possible opportunity.

10.2 FDM buffer limit
The buffer that holds the unsent business transactions is limited in size. It may contain
maximally maxBuffer unsent business transactions. The counter
unsentEventCount holds, at each moment in time, the number of unsent event
transactions in the buffer. When >70% of the buffer memory is used, the user receives a
warning BUFFER_NEAR_FULL via the connected POS systems at the next possible
opportunity. If 100% of the buffer memory is used, the FDM stops accepting incoming business
transactions from the connected POS systems. The user receives an error BUFFER_FULL
that explains the situation via the connected POS systems at the next possible opportunity.
If the buffer of the FDM is full, the user should connect its FDM to the internet so that the unsent
business transactions can be ventilated.
FPS Finance can disable the buffer limit by setting the value maxBuffer = 0 . In that case,
the buffer size is limited by the physical capacity of the FDM device.

10.3 FDM lock
FPS Finance can decide to lock and unlock an FDM device ( fdmLock ). A locked FDM is still
connected to the online service, but will refuse to accept and sign incoming business
transactions from the POS. The user will systematically receive the error lockReason
message (in accordance with the POS language choice).




                                                 22
11. Administration console
The Administration Console of the FDM should at least support the following functionalities in a
stand-alone fashion, without any dependency from the manufacturer.

 Functionality                                                                    User access
Read the error log.                                                               X
Read the buffer transactions history: the percentage of used buffer               X
( unsentEventCount divided by maxBuffer when not = 0),
timestamp of last successful transmission
( fdmLastSuccessfulTrans ) and number of retries since last
send attempt ( retryCount ).
Configure network settings.                                                       X
Read the data elements which are accessible to the user (see section 5).          X

An electronic form to enter the URL of ws_init (see section 12.3).                X

Remark : when the URL of ws_init is changed in the administration
console, if we are in a retryIntervalsInit cycle in the
initialization webservice, we restart the cycle at the first value of the list.
The goal is to have a retry ASAP.
Manually adapt/fill the ws_ntp3 URL of the NTP timeserver.                        X
A button to resync the real-time clock with the NTP timeserver.                   X
Consult the current version of the FDM parameters                                 X
( paramsVersion ).
A button to call the query webservice (download queries list and execute          X
queries if necessary) ws_query


12. Webservices
12.1 General
All webservices of type “Restful Web Services” using JSON payload encoded in UTF-8 over
HTTP 1.1. Authentication and encryption is realized using mTLS 1.2 using a client and server

                                                    23
certificate. As security technology evolves, FPS Finance can require newer versions or other
(security) protocols.
If for some reason the FDM received illegal byte sequence(s) from the POS, it may exceptionally
represent these bytes via a JSON escape sequence \u .
If mTLS authorization fails, the TCP/IP connection may be closed immediately without a reply
from the webservice.


    Important note :
    All the communications from the FDM 2.0 to GKS2 webservices have a limitation of a
    maximun 6MB depending on the limitation from our provider.
    For the Register webservice, the limitation is 1MB.
    All calls to a webservice with a size bigger will return an error..

12.2 Error logging
At least all failed webservice requests and responses (if any) should be logged for a minimum of
5 days, or at least the 10 last failed requests (HTTP headers, JSON payload). The FDM may
deviate from this principle if it runs out of physical storage: loggings may be deleted starting
from the oldest entries. Log items should be accessible for FPS Finance with a message
ws_query.
Others errors, requests that fail due to no internet connection, etc. should not be logged in this
way (but via the generic error log with only a message without content). These log items should
be accessible to the user for inspection via the Administration Console (see section 11).


GraphQL description for error logging:
The error logging FailedRequest :
 1 # The fields for FailedRequests
 2 type FailedRequest {
 3   timestamp: String!         # "2024-04-09T04:00:00Z" for example
 4   requestMethod: String      # POST or GET
 5   requestUri: String         # address of the endpoint for the web service
 6   requestHeaders: String     # the headers of the request in base64
 7   requestBody: String        # the body of the request in base64
 8   replyStatusCode: Integer # the status code obtained from the server (if reached that far)
 9   replyHeaders: String       # the headers obtained from the server in base64 (if reached tha
10   replyBody: String          # the body obtained from the server in base64 (if it was reached
11   errorMessage: String       # A error message
12   data: [DataItem!]          # Generic data object (can be used for POS for example)
                                                24
13 }
14
15 type DataItem {
16     name: String!
17     value: String!
18 }


12.3 Initialization webservice
The initialization webservice provides a bootstrap package for the FDM. It contains the URLs,
trust/claim/client certificates, parameters, and VAT rates.
The user may manually modify the value of ws_init via the Administration Console. The
correct value will be published on the e-service of FPS Finance.
The webservice is exposed via HTTP with TLS without validation of the server certificate, but
with the use of an authorization token. This token is established as a JSON web token using an
Elliptic Curve Digital Signature (ECDSA) and the hardware key fdmHkey . The payload
contains the subject init , the "issued at time" timestamp iat , and the fdmId .
Conditionally, you must add the serial number of the client certificate clientCertSn :
  If the client certificate is nonexistent or expired, the clientCertSn was not provided in
  the request.
  If the client certificate is valid and not expired, the clientCertSn was provided in the
  request.
JWT header:
1 { "alg": "ES256", "typ": "JWT" }

JWT payload in case of nonexistent or expired client certificate:
1 { "sub": "init",
2   "iat": 1686315756,
3   "fdmId": "FDM01234567"
4 }

JWT payload in case of existing, not expired client certificate. We additionally provide the serial
number of the client certificate:
1 { "sub": "init",
2   "iat": 1686315756,
3   "fdmId": "FDM01234567",
4   "clientCertSn": "dhjs76djs"
5 }




                                                 25
    The webservice call should take place immediately after the generation of the token,
    because FPS Finance enforces a very short token lifespan.

Protocol: TLS with authorization token (JWT)
Verb: POST
URL: /init (using ws_init )
12.3.1 Reply with retry invitation
It is possible that FPS Finance can’t synchronously provide an initialization package. In this
case, the reply contains a retry invitation expressed in seconds (HTTP code 200). When the
waiting time has elapsed, the FDM can retry the initialization.
In the following example, the FDM is invited to wait 5 minutes before a retry.
1 { "wait": 300 }

12.3.2 Successful reply
The initialization package is formatted in JSON and transcoded to base64. The reply can be
with or without a new client certificate.
  If the clientCertSn was not provided in the request, FPS Finance will issue a new
  client certificate in the reply.
  If the clientCertSn was provided in the request (in case of an existing client certificate
  that is not expired), FPS Finance will issue a new client certificate in the reply if the previous
  client certificate is retracted. Otherwise, the clientCert element will be empty or not
  present in the reply. This means that the FDM can continue to use its current client certificate.
We show a concrete example of a possible reply. The queries are: SetFdmLock, SetParams,
SetVatRates, SetPosAllowlist, SetTrustCert and SetUrls.
Remarks:
  the actionId is specified only when FPS Finance needs this information in the GraphQL
  reply to avoid resending this query at the next ws_query call.
The SetClientCert query (in case the clientCert needs to be renewed) will be done in the next
call of ws_query.
Sample: (for more information about mutations used in the sample, see 12.5.5.1, 12.5.5.2, 12.5.5.3,
12.5.5.4, 12.5.5.5 & 12.5.5.6)
   1 [
                                                26
 2   {
 3        "dataResultUrl": "https://www.fod/data_endpoint",
 4        "errorResultUrl": "https://www.fod/error_endpoint",
 5        "actionId": "2023-01-05T18:03:23.000Z",
 6        "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 7        "operationType": "SET_FDM_LOCK",
 8        "executable": {
 9            "query": "mutation SetFdmLock($fdmLockFromDateList: [FdmLockInput!]!)
10                        { setFdmLock(fdmLockFromDateList: $fdmLockFromDateList)
11                            {
12                                device { fdmId fdmDateTime bufferCapacityUsed }
13                                configuration { paramsVersionObject { paramsVersion }
14                                fdmLockFromDateList
15                                    {fromSystemDate fdmLock lockReason { language message }
16            "operationName": "SetFdmLock",
17            "variables": {
18                "fdmLockFromDateList": [
19                    {
20                        "fromSystemDate": "2024-06-01",
21                        "fdmLock": false,
22                        "lockReason": null
23                    },
24                    {
25                        "fromSystemDate": "2024-07-01",
26                        "fdmLock": true,
27                        "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN" },
28                                        {"language": "NL", "message": "FDM_LOCKED-NL" },
29                                        {"language": "FR", "message": "FDM_LOCKED-FR" },
30                                        {"language": "DE", "message": "FDM_LOCKED-DE" }]
31                    }
32                ]
33            }
34        }
35   },
36   {
37        "dataResultUrl": "https://www.fod/data_endpoint",
38        "errorResultUrl": "https://www.fod/error_endpoint",
39        "actionId": "2023-01-05T18:03:23.000Z",
40        "messageDateTimeEcho": "2024-04-17T04:00:00Z",
41        "operationType": "SET_PARAMS",
42        "executable": {
43            "query": "mutation SetParams($paramsVersionObject: ParamsVersionObjectInput!)
44                        { setParams(paramsVersionObject: $paramsVersionObject)
45                        {
46                            device { fdmId fdmDateTime bufferCapacityUsed }
47                            configuration { paramsVersionObject { paramsVersion }
48                            fdmLockFromDateList
49                                {fromSystemDate fdmLock lockReason { language message } } }
50            "operationName": "SetParams",
51            "variables": {
52                "paramsVersionObject": {
53                    "paramsVersion": "2023-01-05T18:03:23.000Z",
54                    "paramsArray": [
55                        {
56                            "fromSystemDate": "2024-07-01",
57                            "params": {
58                                "url": "https://www.gs2.be/",
59                                "initInterval": 345600,

                                            27
 60                                 "nopInterval": 21600,
 61                                 "maxBuffer": 20000,
 62                                 "queryInterval": 21600,
 63                                 "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
 64                                 "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 10800],
 65                                 "maxMsgSize": 32,
 66                                 "postpone": true,
 67                                 "defaultMaxCalls": 30,
 68                                 "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "maxCalls
 69                                     {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot": 10
 70                                     {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot": 0}
 71                                 "footer": ["myfootertext1", "myfootertext1"],
 72                                 "vatNo": "1234567890",
 73                                 "estNo": "0987654321",
 74                                 "protocolVersion": "0000000001"
 75                             }
 76                        },
 77                        {
 78                             "fromSystemDate": "2024-08-01",
 79                             "params": {
 80                                 "url": "https://www.gs2.be/",
 81                                 "initInterval": 345600,
 82                                 "nopInterval": 21600,
 83                                 "maxBuffer": 20000,
 84                                 "queryInterval": 21600,
 85                                 "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
 86                                 "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 10800],
 87                                 "maxMsgSize": 32,
 88                                 "postpone": true,
 89                                 "defaultMaxCalls": 30,
 90                                 "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "maxCalls
 91                                     {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot": 10
 92                                     {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot": 0}
 93                                 "footer": ["myfootertext2", "myfootertext2"],
 94                                 "vatNo": "1234567890",
 95                                 "estNo": "0987654321",
 96                                 "protocolVersion": "0000000001"
 97                             }
 98                        }
 99                    ]
100                }
101            }
102        }
103   },
104   {
105        "dataResultUrl": "https://www.fod/data_endpoint",
106        "errorResultUrl": "https://www.fod/error_endpoint",
107        "actionId": "2023-01-05T18:03:23.000Z",
108        "messageDateTimeEcho": "2024-04-17T04:00:00Z",
109        "operationType": "SET_VAT_RATES",
110        "executable": {
111            "query": "mutation SetVatRates($vatFromBookingDateList: [VatRatesInput!]!)
112                        { setVatRates(vatFromBookingDateList: $vatFromBookingDateList)
113                        {
114                            device { fdmId fdmDateTime bufferCapacityUsed }
115                            configuration { paramsVersionObject { paramsVersion }
116                            fdmLockFromDateList
117                                {fromSystemDate fdmLock lockReason { language message } } }

                                              28
118   "operationName": "SetVatRates",
119   "variables": {
120       "vatFromBookingDateList": [
121           {
122           "fromBookingDate": "2020-01-01",
123           "vatRates": [
124               {
125                   "vatLabel": "A",
126                   "vatRate": 21,
127                   "outOfScope": false
128               },
129               {
130                   "vatLabel": "B",
131                   "vatRate": 12,
132                   "outOfScope": false
133               },
134               {
135                   "vatLabel": "C",
136                   "vatRate": 6,
137                   "outOfScope": false
138               },
139               {
140                   "vatLabel": "D",
141                   "vatRate": 0,
142                   "outOfScope": false
143               },
144               {
145                   "vatLabel": "X",
146                   "vatRate": 0,
147                   "outOfScope": true
148               }
149           ]
150           },
151           {
152           "fromBookingDate": "2025-04-01",
153           "vatRates": [
154               {
155                   "vatLabel": "A",
156                   "vatRate": 21,
157                   "outOfScope": false
158               },
159               {
160                   "vatLabel": "B",
161                   "vatRate": 17,
162                   "outOfScope": false
163               },
164               {
165                   "vatLabel": "C",
166                   "vatRate": 6,
167                   "outOfScope": false
168               },
169               {
170                   "vatLabel": "D",
171                   "vatRate": 0,
172                   "outOfScope": false
173               },
174               {
175                   "vatLabel": "X",

                                    29
176                            "vatRate": 0,
177                            "outOfScope": true
178                        }
179                    ]
180                    }
181                ]
182            }
183        }
184   },
185   {
186        "dataResultUrl": "https://www.fod/data_endpoint",
187        "errorResultUrl": "https://www.fod/error_endpoint",
188        "actionId": "2023-01-05T18:03:23.000Z",
189        "messageDateTimeEcho": "2024-04-17T04:00:00Z",
190        "operationType": "SET_POS_ALLOWLIST",
191        "executable": {
192            "query": "mutation SetPosAllowlist($posIdFromDateList: [PosAllowlistInput!]!)
193                        { setPosAllowlist (posIdFromDateList: $posIdFromDateList)
194                        {
195                            device { fdmId fdmDateTime bufferCapacityUsed }
196                            configuration { paramsVersionObject { paramsVersion }
197                            fdmLockFromDateList {fromSystemDate fdmLock lockReason { languag
198                            posIdFromDateList { fromSystemDate posIdList } } } }",
199            "operationName": "SetPosAllowlist",
200            "variables": {
201                "posIdFromDateList": [
202                    {
203                        "fromSystemDate": "2024-07-01",
204                        "posIdList": ["Csnpos00000001", "Csnpos00000003"]
205                    },
206                    {
207                        "fromSystemDate": "2024-08-01",
208                        "posIdList": ["Csnpos00000001", "Csnpos00000003", "Csnpos00000004"]
209                    }
210                ]
211            }
212        }
213   },
214   {
215        "dataResultUrl": "https://www.fod/data_endpoint",
216        "errorResultUrl": "https://www.fod/error_endpoint",
217        "actionId": "2023-01-05T18:03:23.000Z",
218        "messageDateTimeEcho": "2024-04-17T04:00:00Z",
219        "operationType": "SET_TRUST_CERT",
220        "executable": {
221            "query": "mutation SetTrustCert($trustCertFromDateList: [TrustCertInput!]!)
222                        { setTrustCert(trustCertFromDateList: $trustCertFromDateList)
223                        {
224                            device { fdmId fdmDateTime bufferCapacityUsed }
225                            configuration { paramsVersionObject { paramsVersion }
226                            fdmLockFromDateList
227                                {fromSystemDate fdmLock lockReason { language message } } }
228            "operationName": "SetTrustCert",
229            "variables": {
230                "trustCertFromDateList": [
231                    {
232                        "fromSystemDate": "2024-07-01",
233                        "trustCert": [ "cert1x509pem", "cert2x509pem", "cert3x509pem" ]

                                             30
234                       },
235                       {
236                            "fromSystemDate": "2024-08-01",
237                            "trustCert": [ "cert1x509pem2", "cert2x509pem2", "cert3x509pem2" ]
238                       }
239                   ]
240               }
241           }
242      },
243      {
244           "dataResultUrl": "https://www.fod/data_endpoint",
245           "errorResultUrl": "https://www.fod/error_endpoint",
246           "actionId": "2023-01-05T18:03:23.000Z",
247           "messageDateTimeEcho": "2024-04-17T04:00:00Z",
248           "operationType": "SET_URLS",
249           "executable": {
250               "query": "mutation SetUrls($urlsFromDateList: [UrlInput!]!)
251                           { setUrls(urlsFromDateList: $urlsFromDateList)
252                           {
253                               device { fdmId fdmDateTime bufferCapacityUsed }
254                               configuration { paramsVersionObject { paramsVersion }
255                               fdmLockFromDateList
256                                   {fromSystemDate fdmLock lockReason { language message } } }
257               "operationName": "SetUrls",
258               "variables": {
259                   "urlsFromDateList": [
260                       {
261                           "fromSystemDate": "2024-07-01",
262                           "urls": [
263                               { "key": "ws_reg", "value": "https://myurl:8080" },
264                               { "key": "ws_nop", "value": "https://myurl:8080" },
265                               { "key": "ws_query", "value": "https://myurl:8080" },
266                               { "key": "ws_init", "value": "https://myurl:8080" },
267                               { "key": "ws_ntp1", "value": "https://myurl:8080" },
268                               { "key": "ws_ntp2", "value": "https://myurl:8080" }
269                           ]
270                       },
271                       {
272                           "fromSystemDate": "2024-08-01",
273                           "urls": [
274                               { "key": "ws_reg", "value": "https://myurl2:8080" },
275                               { "key": "ws_nop", "value": "https://myurl2:8080" },
276                               { "key": "ws_query", "value": "https://myurl2:8080" },
277                               { "key": "ws_init", "value": "https://myurl2:8080" },
278                               { "key": "ws_ntp1", "value": "https://myurl2:8080" },
279                               { "key": "ws_ntp2", "value": "https://myurl2:8080" }
280                           ]
281                       }
282                   ]
283               }
284           }
285      }
286 ]

The full reply contains the current clientCert (empty value when clientCertSn is
specified in the JWT), the initialization package in base64 encoding, the digital signature [of the

                                                 31
client certificate as "pem" + "init-package-in-base64" ] and the claim
certificates. The electronic signature is established with SHA-256 and Elliptic Curve Digital
Signature (ECDSA) in base64 output using the private key of the claim certificate. The claim
certificate and its intermediates are specific to an FDM vendor (see section 6.2). The PKCS12
contains the client certificate and the private key (encryption password = secret, details can be
requested from the relevant department). The pem , used for the digital signature, contains
only the certificate.
When the clientCertSn is not specified:
1 { "clientCert": "pkcs12pem-with-password", "init": "init-package-in-base64", "digitalSignature
2 "claim": [ "cert1x509pem", "cert2x509pem" ] }

When the clientCertSn is specified:
1 { "clientCert": "", "init": "init-package-in-base64", "digitalSignature": "hjd76gd5",
2 "claim": [ "cert1x509pem", "cert2x509pem" ] }

The FDM should perform the following steps:
  Validate the chain of claim certificates using the root certificate that is built into the firmware.
  Validate the digital signature using the claim certificate. To this end, SHA-256 and ECDSA
  with base64 output should be used.
  If the validation of the claim certificates and digital signature is successful, the FDM is
  allowed to install the initialization package.
  First, the client certificate from the initialization package can be imported using the shared
  secret and FDM serial number.
  Then the queries are executed (see 12.5 for more information).
  Then the FDM calls ws_query to receive pending queries and execute them (if there are
  any).
12.3.3 Exceptions
  HTTP code 400/404: Bad Request/General error
  HTTP code 401/403: Authorization token is invalid
  HTTP code 404: Page not found (bad ws_init URL)
  HTTP code 406: FPS Finance refuses to issue a new client certificate (normally will never
  happen, we use the fdmLock parameter)
In case of an unsuccessful execution of the initialization webservice, the warning
 INITIALIZATION_FAILED should be shown on the linked POS systems at the next
                                                  32
possible opportunity.
If a retry of the initialization webservice is needed (bad client certificate, bad claim
certificates,…), to avoid a loop in the initialization webservice calls, the
 retryIntervalsInit must be applied by the FDM before next call. The parameter
retryIntervalsInit specifies a series of time intervals expressed in seconds at which
a retry should take place after a failed call to the initialization webservice. If the list of retry
intervals is exhausted due to persistent failed calls, the last value may be repeated. Without
availability of retryIntervalsInit , the FDM must use a interval of 600 seconds (10
minutes) before next call.
Remark : when the URL of ws_init is changed in the administration console, if we are in a
retryIntervalsInit cycle in the initialization webservice, we restart the cycle at the
first value of the list. The goal is to have a retry ASAP (see section 11).

12.4 Register webservice
The business transactions and their electronic signatures, which are stored in the FDM buffer
(see section 9.1), should be dispatched to FPS Finance using the register webservice.
12.4.1 Register business transactions
The payload of the register webservice contains the following elements:
  The size of the FDM buffer unsentEventCount at the moment of the webservice call.
  The fdmId.
  The vatNo and estNo from FDM params values (mandatory) at the moment of the
  webservice call.
    An exception to this rule : the Emergency FDM, uses the estNo & vatNo of the
    values inside the business transactions from the POS (no need to call ws query before use
    in a businesses).
  The protocolVersion at the moment of the webservice call.
  An array of business transaction events . Each array element contains a single business
  transaction. By default, only one business transaction per webservice call is preferred,
  although multiple business transactions in one call is permitted. If postpone = true ,
  multiple business transactions must be sent ( maxMsgSize , see 9.2.1 for more details) in a
  single webservice call. The maximum size of the body payload is determined by the
   maxMsgSize parameter.
                                                   33
  a single business transaction consists of the detailed transaction description
  ( enrichedEventData ), the electronic signature in base64
  ( digitalSignature ), a short signature for printed ticket ( shortSignature ), an
  url( verificationUrl ) and the localisation information ( fdmLocalisation ).
  When a read transaction is corrupted (due to bad or defective sectors in the memory) or
  becomes unreadable in the memory, the FDM replaces the standard business transaction with
   rawCounter (required), rawBytes (optional) and rawInfo (required).
   rawCounter is a counter incremented every time the FDM sends a corrupt transaction to
  the FPS Cloud. rawBytes is sent when the corrupt transaction is readable. rawInfo is
  a comment/info of the FDM. The warning CORRUPT_RECORD_ENCOUNTERED should
  be shown on the linked POS systems at the next possible opportunity. If the issue
  ( CORRUPT_RECORD_ENCOUNTERED ) persists, FPS Finance can decide to set
   fdmLock to true with a lockReason message. All incoming business transactions will
  then be refused.
You need a valid client certificate certClient for calling this webservice (see section 6.3).
The FDM must have a value for fields estNo and vatNo (not null, not empty) to fill the
field in the json (see below) for calling this webservice. If this is not the case, business
transactions remain in the buffer. The estNo and vatNo are updated by SetParams
mutation (see 12.5.5.2 Mutation: SetParams). An exception to this rule, the Emergency FDM,
uses the estNo & vatNo of the values inside the business transactions from the POS (no
need to call ws query before use in a businesses).
The conditions under which the FDM is allowed to call this webservice are described in section
9.2.
Protocol: mTLS using certClient
Verb: POST
URL: /reg (using ws_reg )
Example request looks as follows:
 1 {
 2      "unsentEventCount": 999,
 3      "fdmId": "FOD01000001",
 4      "vatNo": "1234567890",
 5      "estNo": "0987654321",
 6      "protocolVersion": "0000002.00",
                                                34
 7      "events": [
 8          {
 9              "enrichedEventData": { ... },
10              "digitalSignature": "base64h2f4sh1",
11              "shortSignature": "sha-1",
12              "verificationUrl": "url",
13              "fdmLocalisation": {
14                  "geoLocation": {
15                      "latitude": 50.083207,
16                      "longitude": 5.417244
17                  }
18              }
19          },
20          {
21              "enrichedEventData": { ... },
22              "digitalSignature": "base64h2f4sh2",
23              "shortSignature": "sha-1",
24              "verificationUrl": "url",
25              "fdmLocalisation": {
26              }
27          },
28          {
29              "enrichedEventData": { ... },
30              "digitalSignature": "base64h2f4sh3",
31              "shortSignature": "sha-1",
32              "verificationUrl": "url",
33              "fdmLocalisation": {
34              }
35          }
36      ]
37 }




Example request with corrupt transaction in memory:
 1 {
 2      "unsentEventCount": 999,
 3      "fdmId": "FOD01000001",
 4      "vatNo": "1234567890",
 5      "estNo": "0987654321",
 6      "protocolVersion": "0000002.00",
 7      "events": [
 8          {
 9              "enrichedEventData": { ... },
10              "digitalSignature": "base64h2f4sh1",
11              "shortSignature": "sha-1",
12              "verificationUrl": "url",
13              "fdmLocalisation": {
14                  "geoLocation": {
15                      "latitude": 50.083207,
16                      "longitude": 5.417244
17                  }
18              }
19          },
20          {
21              "rawCounter": 1,
22              "rawBytes": "base64h2f4sh1",
                                               35
23                "rawInfo": "eventueel opmerking door fdm"
24           },
25           {
26                "rawCounter": 2,
27                "rawInfo": "eventueel opmerking door fdm"
28           },
29           {
30                "enrichedEventData": { ... },
31                "digitalSignature": "base64h2f4sh2",
32                "shortSignature": "sha-1",
33                "verificationUrl": "url",
34                "fdmLocalisation": {
35                }
36           },
37           {
38                "enrichedEventData": { ... },
39                "digitalSignature": "base64h2f4sh3",
40                "shortSignature": "sha-1",
41                "verificationUrl": "url",
42                "fdmLocalisation": {
43                }
44           }
45      ]
46 }




If the business transactions call fails, the warning TRANSACTION_UPLOAD_FAILED
should be shown on the linked POS systems at next possible opportunity.
12.4.2 No Operation message
The payload can potentially be empty in case of a “No Operation” (NOP) message which is sent
at frequent intervals (see section 9.2). Example of a NOP:
Protocol: mTLS using certClient
Verb: POST
URL: /reg (using ws_nop )
1 { "unsentEventCount": 0,
2   "fdmId": "FOD01000001",
3   "vatNo": "1234567890",
4   "estNo": "0987654321",
5   "protocolVersion": "0000002.00",
6   "events": []
7 }

If the NOP call fails, the warning NOP_FAILED should be shown on the linked POS systems
at next possible opportunity.


                                                 36
12.4.3 Replies

HTTP code 200 indicates a successful call. The value of fdmLastSuccessfulTrans
should be updated and the counter should be reset.
All other replies different than HTTP code 200 indicate an error. In that case, the counter
 retryCount should be incremented. If retryCount > 10 , the user should receive
warning TRANSACTION_UPLOAD_FAILED on the connected POS systems at the next
possible opportunity.
The parameter retryIntervals specifies a series of time intervals expressed in seconds
at which a retry should take place after a failed call to the register webservice. If the list of retry
intervals is exhausted due to persistent failed calls, the last value may be repeated. Retries are
not necessary for failed NOP messages.

    The retry policy may slow down the transmission frequency (see section 9.2). FPS Finance
    will configure the retryIntervals with caution.

12.5 Query webservice
FPS Finance can issue queries to be executed by the FDM. The FDM should call the query
webservice ( ws_query ) at each queryInterval to check if there are queries to be
done (when fdmDateTime > fdmLastActions + queryInterval ).
FPS Finance always sends all the queries to be executed at that time (array of queries). If FPS
Finance doesn’t receive the response to a query, the query will be resent at the next call to
 ws_query (after queryInterval ). So, after queryInterval , the FDM can drop
the not already done reply of the previous ws_query call.
Each query should be executed by the FDM as soon as possible.


The queries are written in JSON GraphQL compatible format for compatibility with GraphQL on
the FDM.
Next, once a query is executed (with or without success), the FDM should reply to
 dataResultUrl (success) or errorResultUrl (errors). This is explained in sections
12.5.2. & 12.5.3.
If needed, in the future, FPS Finance can add new queries (mutation or query). In that case, a
firmware update is needed.
                                                  37
The FDM accepts only these described queries from the FPS Finance query webservice
( ws_query ). No other queries can be used to modify the parameters described in these
queries (mutations). More info about an update of the parameters can be found in section 5.
12.5.1 Get the queries to-do list

Protocol: mTLS with certClient
Verb: GET
URL: /query (using ws_query )
The possible queries are:
  Mutation: SetClientCert : the FDM is requested to renew its client certificate
   certClient as soon as possible. See section 12.5.5.7 of this procedure.
  Mutation: SetTrustCert : the FDM is requested to fetch the chain of trust certificates
  related to the server certificate as soon as possible. See section 6.4 and 12.5.5.5 for more info.
  Mutation: SetParams : the FDM is requested to renew its parameters as soon as possible.
  See section 12.5.5.2 for the details of this procedure.
  Mutation: SetUrls : the FDM is requested to renew the URLs of the webservices as soon
  as possible. See section 12.5.5.6 for the details of this procedure.
  Mutation: SetVatRates : the FDM is requested to renew the VAT rates as soon as
  possible. See section 12.5.5.3 for the details of this procedure.
  Mutation: SetFdmLock : the FDM is requested to set the fdmLock as soon as
  possible. See section 12.5.5.1 for the details of this procedure.
  Mutation: SetPosAllowlist : the FDM is requested to set the posIdList as soon
  as possible. See section 12.5.5.4 for the details of this procedure.
  Query: FailedRequests : the FDM is requested to send failed request as soon as
  possible. See section 12.5.5.8 for the details of this procedure.
  Query: FdmConfiguration : the FDM is requested to send all FDM configurations as
  soon as possible. See section 12.5.5.9 for the details of this procedure.
An query is always announced with a actionId timestamp, so that we can refer to a
particular action in the FPS system. There is no fixed order in which the queries appear in the
list.
                                                38
A success call to the ws_query implies an update of fdmLastActions timestamp.


When an error occurs and the FDM can't receive the queries to-do list, Warning
 UPDATE_TASK_LIST_FAILED should be shown on the linked POS systems at the next
possible opportunity.
If the error concerns a problem with the client certificate or the trust certificate (http code 401,
403) of this webservice, or a bad URL (http codes 400, 404), set fdmInit = true , call
the initialization webservice (ws_init) and install the initialization package (see section 12.3). If
this call fails persistently, the owner might have to verify, and update if needed, the URL of
 ws_init manually in the Administration Console. After successful execution of the
initialization webservice, set fdmInit = false and update the fdmLastActions
timestamp.
For other errors the FDM applies the retryIntervals to call the query webservice. The
parameter retryIntervals specifies a series of time intervals expressed in seconds at
which a retry should take place after a failed call to the webservice. If the list of retry intervals is
exhausted due to persistent failed calls, the FDM evaluates the condition to execute a call to the
initialization webservice (ws_init): If fdmDateTime > fdmLastActions +
initInterval , the FDM sets fdmInit = true , calls the initialization webservice
(ws_init) and installs the initialization package (see section 12.3). If this call fails persistently, the
owner might have to verify, and update if needed, the URL of ws_init manually in the
Administration Console. After successful execution of the initialization webservice, set
 fdmInit = false and update the fdmLastActions timestamp.
Otherwise, the FDM continues the try again and the last value of the retryIntervals
may be repeated until a successful call to query webservice or the condition to call the
initialization webservice is reached.
12.5.2 Queries root structure
  Query samples
    One query:
       1 [
       2       {
       3            "dataResultUrl": "https://www.fod/data_endpoint",
       4            "errorResultUrl": "https://www.fod/error_endpoint",
       5            "actionId": "2023-01-05T18:03:23.000Z",
                                                    39
 6           "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 7           "operationType": "SET_FDM_LOCK",
 8           "executable": {
 9               "query": "mutation SetFdmLock($fdmLockFromDateList: [FdmLockInput!]!)
10                           { setFdmLock(fdmLockFromDateList: $fdmLockFromDateList)
11                           {
12                               device { fdmId fdmDateTime bufferCapacityUsed }
13                               configuration { paramsVersionObject { paramsVersion }
14                               fdmLockFromDateList
15                                   {fromSystemDate fdmLock lockReason { language message }
16               "operationName": "SetFdmLock",
17               "variables": {
18                   "fdmLockFromDateList": [
19                       {
20                           "fromSystemDate": "2024-06-01",
21                           "fdmLock": false,
22                           "lockReason": null
23                       },
24                       {
25                           "fromSystemDate": "2024-07-01",
26                           "fdmLock": true,
27                           "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN" },
28                           {"language": "NL", "message": "FDM_LOCKED-NL" },
29                           {"language": "FR", "message": "FDM_LOCKED-FR" },
30                           {"language": "DE", "message": "FDM_LOCKED-DE" }]
31                       }
32                   ]
33               }
34           }
35      }
36 ]

two queries:
  1 [
  2      {
  3            "dataResultUrl": "https://www.fod/data_endpoint",
  4            "errorResultUrl": "https://www.fod/error_endpoint",
  5            "actionId": "2023-01-05T18:03:23.000Z",
  6            "messageDateTimeEcho": "2024-04-17T04:00:00Z",
  7            "operationType": "SET_FDM_LOCK",
  8            "executable": {
  9                "query": "mutation SetFdmLock($fdmLockFromDateList: [FdmLockInput!]!)
 10                            { setFdmLock(fdmLockFromDateList: $fdmLockFromDateList)
 11                            {
 12                                device { fdmId fdmDateTime bufferCapacityUsed }
 13                                configuration { paramsVersionObject { paramsVersion }
 14                                fdmLockFromDateList
 15                                    {fromSystemDate fdmLock lockReason { language message }
 16                "operationName": "SetFdmLock",
 17                "variables": {
 18                    "fdmLockFromDateList": [
 19                        {
 20                            "fromSystemDate": "2024-06-01",
 21                            "fdmLock": false,
 22                            "lockReason": null
 23                        },
 24                        {

                                            40
25                        "fromSystemDate": "2024-07-01",
26                        "fdmLock": true,
27                        "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN" },
28                        {"language": "NL", "message": "FDM_LOCKED-NL" },
29                        {"language": "FR", "message": "FDM_LOCKED-FR" },
30                        {"language": "DE", "message": "FDM_LOCKED-DE" }]
31                    }
32                ]
33            }
34        }
35   },
36   {
37        "dataResultUrl": "https://www.fod/data_endpoint",
38        "errorResultUrl": "https://www.fod/error_endpoint",
39        "actionId": "2023-01-05T18:03:23.000Z",
40        "messageDateTimeEcho": "2024-04-17T04:00:00Z",
41        "operationType": "SET_PARAMS",
42        "executable": {
43            "query": "mutation SetParams($paramsVersionObject: ParamsVersionObjectInput
44                        { setParams(paramsVersionObject: $paramsVersionObject)
45                        {
46                            device { fdmId fdmDateTime bufferCapacityUsed }
47                            configuration { paramsVersionObject { paramsVersion }
48                            fdmLockFromDateList
49                                {fromSystemDate fdmLock lockReason { language message }
50            "operationName": "SetParams",
51            "variables": {
52                "paramsVersionObject": {
53                    "paramsVersion": "2023-01-05T18:03:23.000Z",
54                    "paramsArray": [
55                        {
56                            "fromSystemDate": "2024-07-01",
57                            "params": {
58                                "url": "https://www.gs2.be/",
59                                "initInterval": 345600,
60                                "nopInterval": 21600,
61                                "maxBuffer": 20000,
62                                "queryInterval": 21600,
63                                "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
64                                "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 1080
65                                "maxMsgSize": 32,
66                                "postpone": true,
67                                "defaultMaxCalls": 30,
68                                "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "max
69                                {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot": 1
70                                {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot": 0
71                                "footer": ["myfootertext1", "myfootertext1"],
72                                "vatNo": "1234567890",
73                                "estNo": "0987654321",
74                                "protocolVersion": "0000000001"
75                            }
76                        },
77                        {
78                            "fromSystemDate": "2024-08-01",
79                            "params": {
80                                "url": "https://www.gs2.be/",
81                                "initInterval": 345600,
82                                "nopInterval": 21600,

                                       41
       83                                    "maxBuffer": 20000,
       84                                    "queryInterval": 21600,
       85                                    "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
       86                                    "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 1080
       87                                    "maxMsgSize": 32,
       88                                    "postpone": true,
       89                                    "defaultMaxCalls": 30,
       90                                    "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "max
       91                                    {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot": 1
       92                                    {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot": 0
       93                                    "footer": ["myfootertext2", "myfootertext2"],
       94                                    "vatNo": "1234567890",
       95                                    "estNo": "0987654321",
       96                                    "protocolVersion": "0000000001"
       97                                }
       98                            }
       99                        ]
      100                    }
      101                }
      102           }
      103       }
      104 ]




  Query root description
    dataResultUrl: reply endpoint for success - mandatory
    errorResultUrl: reply endpoint for errors - mandatory
    executable: the core executable query in GraphQL mutation format (for compatibility with
    GraphQL on the FDM) - mandatory
       query: query in JSON GraphQL mutation format or query format - mandatory
       operationName: Name of the query - mandatory
       variables: to put data for the query & mutation, if needed - mandatory
    All other fields in the root are considered as “echo” and will be copied as is in the reply (in
    the sample: “actionId” & “messageDateTimeEcho”) - optional fields in the queries


12.5.3 FDM reply structure
  Reply sample success:
    1 {
    2       "actionId": "2023-01-05T18:03:23.000Z",
    3       "messageDateTimeEcho": "2024-04-17T04:00:00Z",
    4       "operationType": "SET_FDM_LOCK",
    5       "result": {
    6           "data": {
    7               "setFdmLock": {
    8                   "device": {
    9                       "fdmId": "FOD01000001",
                                                  42
10                        "fdmDateTime": "2024-04-18T04:00:05Z",
11                        "bufferCapacityUsed": 35.12,
12                    },
13                    "configuration": {
14                        "paramsVersionObject": {
15                            "paramsVersion": "2023-01-05T18:03:23.000Z"
16                        },
17                        "fdmLockFromDateList": [
18                            {
19                                "fromSystemDate": "2024-06-01",
20                                "fdmLock": false,
21                                "lockReason": null
22                            },
23                            {
24                                "fromSystemDate": "2024-07-01",
25                                "fdmLock": true,
26                                "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN" },
27                                {"language": "NL", "message": "FDM_LOCKED-NL" },
28                                {"language": "FR", "message": "FDM_LOCKED-FR" },
29                                {"language": "DE", "message": "FDM_LOCKED-DE" }]
30                            }
31                        ]
32                    }
33              }
34          }
35      }
36 }

Reply sample error:
 1 {
 2      "actionId": "2023-01-05T18:03:23.000Z",
 3      "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 4      "operationType": "SET_FDM_LOCK",
 5      "result": {
 6          "errors": [
 7              {
 8                  "message": "A message",
 9                  "extensions": {
10                      "fdmId": "FOD01000001",
11                      "timestamp": "2024-04-17T04:00:00Z"
12                  }
13              }
14          ]
15      }
16 }

Reply description:
  actionId: echo field - optional (depending on the query)
  messageDateTimeEcho: echo field - optional (depending on the query)
  result: field for data or errors- mandatory
    data: field for mutation replies - mandatory
       fields for mutation replies depending on the query - optional

                                              43
      errors: replace data when something wrong occurs during the execution of the query by
      the FDM.
        message: message sent by FDM - mandatory
        extensions: specific fields - mandatory
           fdmId: fdmId - mandatory
           timestamp: error timestamp - mandatory


GraphQL Type system
 1   ##### Mutation #####
 2   type Mutation {
 3     setVatRates(vatFromBookingDateList: [VatRateInput!]!): ConfigurationResult
 4     setParams(paramsVersionObject: ParamsVersionObjectInput!): ConfigurationResult
 5     setFdmLock(fdmLockFromDateList: [FdmLockInput!]!): ConfigurationResult
 6     setPosAllowlist(posIdFromDateList: [PosAllowlistInput!]!): ConfigurationResult
 7     setTrustCert(trustCertFromDateList: [TrustCertInput!]!): ConfigurationResult
 8     setUrls(urlsFromDateList: [UrlInput!]!): ConfigurationResult
 9     setClientCert(clientCertFromDateList: [ClientCertInput!]!): ConfigurationResult
10   }
11
12   ##### Query #####
13   type Query {
14     failedRequests(selectRange: DateTimeRangeInput!): [FailedRequest]
15     allFdmInfos: AllFdmInfos!
16   }
17
18   ################################################## OUTPUT Fields ###########################
19   # The fields for AllFdmInfos
20   type AllFdmInfos {
21     device: Device!
22     transactionState: TransactionState!
23     taskState: TaskState!
24     configuration: Configuration!
25   }
26
27   # The fields for ConfigurationResult
28   type ConfigurationResult {
29       device: Device!
30       configuration: Configuration!
31   }
32
33
34   # The fields for device
35   type Device {
36       fdmId: String!
37       fdmSwVersion: String!
38       fdmDateTime: String!
39       bufferCapacityUsed: Float!
40   }
41
42   # The fields for transactionState
43   type TransactionState {
                                               44
 44       unsentEventCount: Int!         # number of new transactions waiting for upload
 45       fdmLastSuccessfulTrans: String # date and time of the last successful upload
 46       retryCount: Int!               # number of attempts taken to upload the next batch of n
 47   }
 48
 49   # The fields for taskState
 50   type TaskState {
 51       lastDownload: String # date and time of the last successful download [=fdmLastActions].
 52   }
 53
 54   # The fields for Configuration
 55   type Configuration {
 56     fdmLockFromDateList: [FdmLock!]
 57     paramsVersionObject: ParamsVersionObject
 58     vatFromBookingDateList: [VatRates!]
 59     posIdFromDateList: [PosAllowlist!]
 60     trustCertFromDateList: [TrustCert!]!
 61     urlsFromDateList: [Urls!]!
 62     clientCertFromDateList: [ClientCert!]!
 63     localConfig: LocalConfig!
 64   }
 65
 66   # The fields for local config
 67   type LocalConfig {
 68       ws_ntp3: String
 69   }
 70
 71
 72   # The fields for paramsVersionObject
 73   type ParamsVersionObject {
 74       paramsVersion: String!    # date time
 75       paramsArray: [ParamsFromSystemDate!]
 76   }
 77
 78   # The fields for paramsVersionObject
 79   type ParamsFromSystemDate {
 80       fromSystemDate: String! # YYYY-MM-DD
 81       params: Params!
 82   }
 83
 84   type Params {
 85     url: String!
 86     initInterval: Integer!
 87     nopInterval: Integer!
 88     maxBuffer: Integer!
 89     queryInterval: Integer!
 90     retryIntervals: [Integer!]!
 91     retryIntervalsInit: [Integer!]!
 92     maxMsgSize: Integer!
 93     postpone: Boolean!
 94     defaultMaxCalls: Integer!
 95     maxCalls: [MaxCalls!]!
 96     footer: [String!]!
 97     vatNo: String
 98     estNo: String
 99     protocolVersion: String!
100   }
101

                                                   45
102   type MaxCalls {
103     dayOfWeek: WEEKDAY!
104     hour: Integer!           # range 0 - 23.
105     maxCallsInSlot: Integer! # The maximum number of calls allowed in the specified time slot.
106   }
107
108   enum WEEKDAY {
109       MONDAY
110       TUESDAY
111       WEDNESDAY
112       THURSDAY
113       FRIDAY
114       SATURDAY
115       SUNDAY
116   }
117
118   # The fields for posIdFromDateList
119   type PosAllowlist
120   {
121       fromSystemDate: String! # YYYY-MM-DD
122       posIdList: [String]!
123   }
124
125   # The fields for urlsFromDateList
126   type Urls {
127       fromSystemDate: String! # YYYY-MM-DD
128       urls: [KeyValue!]!
129   }
130   type KeyValue {
131      key: String!
132      value: String!
133   }
134
135   # The fields for vatFromBookingDateList
136   type VatRates {
137       fromBookingDate: String! # YYYY-MM-DD
138       vatRates: [VatRate!]!
139   }
140   type VatRate {
141     vatLabel: VatLabel!
142     vatRate: float!
143     outOfScope: Boolean!
144   }
145   enum VatLabel {
146     A
147     B
148     C
149     D
150     X
151   }
152
153   # The fields for fdmLockFromDateList
154   type FdmLock {
155     fromSystemDate: String # YYYY-MM-DD
156     fdmLock: Boolean!
157     lockReason: [Message!]
158   }
159

                                                  46
160   type Message {
161     language: Language!
162     message: String!
163   }
164
165   # The fields for trustCertFromDateList
166   type TrustCert {
167     fromSystemDate: String! # YYYY-MM-DD
168     trustCert: [String!]!
169   }
170
171   # The fields for clientCertFromDateList
172   type ClientCert {
173       fromSystemDate: String! # YYYY-MM-DD
174       clientCertSn: String!
175   }
176
177
178   # The fields for failedRequests
179   see 12.2
180
181
182
183   ############################## All the queries get an enum name so they can be echoed back i
184   enum OperationTypes {
185     SET_POS_ALLOWLIST
186     SET_VAT_RATES
187     SET_URLS
188     SET_TRUST_CERT
189     SET_PARAMS
190     SET_FDM_LOCK
191     SET_CLIENT_CERT
192     FAILED_REQUESTS
193     ALL_FDM_INFOS
194   }
195




12.5.4 Replies executed queries
For each query execution, successful or not, the FDM should send a reply to FPS Finance. See
section 12.5.3 for the root structure of the reply and in the queries description.
Protocol: mTLS with certClient
Verb: POST
URL: dataResultUrl or errorResultUrl


Possible replies are:
  HTTP code 200: the replies have been processed successfully.

                                                 47
  All other replies indicate an error. Warning TASK_FEEDBACK_FAILED should be shown
  on the linked POS systems at the next possible opportunity. At the next queries cycle,
  determined by queryInterval , the FDM should retry to send the reply of successful
  queries on this new cycle.

12.5.5 Queries description
12.5.5.1 Mutation: SetFdmLock

This query sets the fdmLock boolean, the fromSystemDate and the lockReason
array of MessageInput . If fdmLock = true, the device is locked and will not accept and
sign business transactions anymore and the error lockReason message should be shown
on the linked POS systems at the next possible opportunity (in accordance with the POS
language choice).
The fromSystemDate value(s) indicate when the other values in the object are applied.
There can be many fromSystemDate because it’s an array. When the FDM receives this
array, it applies the values that are in the object with the fromSystemDate that
corresponds to the current date. After that, when the FDM reaches another
 fromSystemDate , it applies the values that are in this object.
For example:
   "fromSystemDate": "2024-06-01" → object1
   "fromSystemDate": "2024-07-01" → object2
   The FDM receives this array from the ws_query on “2024-06-10”, so the FDM applies the
   values in object1 directly after installation.
   On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
   In the previous example, FPS Finance must send an object where the
    fromSystemDate covers the current date.
   At the writing date of the query, FPS Finance enters a value for the writing date and values
   for other dates in the future if necessary.
   It is not possible to have this example if the date is 2024-05-19.

                                                48
When the FDM receives this mutation, it installs the new values in his memory and in case of
successful installation (for all fromSystemDate values) drops all the old values. The new
values replace the old ones.
FPS Finance doesn’t require a history of installed values, but the manufacturers are free to
create one.


  Json query from ws_query
    GraphQL Type system:
      1   # +++++ Data in the mutation ++++
      2
      3   # The fields for FDM lock
      4   input FdmLockInput {
      5     fromSystemDate: String! # YYYY-MM-DD
      6     fdmLock: Boolean!
      7     lockReason: [MessageInput!]
      8   }
      9
     10   input MessageInput {
     11     language: Language!
     12     message: String!
     13   }
     14
     15   enum Language {
     16      EN
     17      NL
     18      FR
     19      DE
     20   }

     Query sample:
      1 {
      2   "dataResultUrl": "https://www.fod/data_endpoint",
      3   "errorResultUrl": "https://www.fod/error_endpoint",
      4   "actionId": "2023-01-05T18:03:23.000Z",
      5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
      6   "operationType": "SET_FDM_LOCK",
      7   "executable": {
      8      "query": "mutation SetFdmLock($fdmLockFromDateList: [FdmLockInput!]!)
      9                    { setFdmLock(fdmLockFromDateList: $fdmLockFromDateList)
     10                        {
     11                            device { fdmId fdmDateTime bufferCapacityUsed }
     12                            configuration { paramsVersionObject { paramsVersion }
     13                            fdmLockFromDateList
     14                                { fromSystemDate fdmLock lockReason { language message }
     15      "operationName": "SetFdmLock",
     16      "variables": {
     17        "fdmLockFromDateList": [
     18            {
     19                "fromSystemDate": "2024-06-01",
     20                "fdmLock": false,
                                                   49
  21                  "lockReason": null
  22             },
  23             {
  24                  "fromSystemDate": "2024-07-01",
  25                  "fdmLock": true,
  26                  "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN" },
  27                      {"language": "NL", "message": "FDM_LOCKED-NL" },
  28                      {"language": "FR", "message": "FDM_LOCKED-FR" },
  29                      {"language": "DE", "message": "FDM_LOCKED-DE" }]
  30             }
  31       ]
  32     }
  33   }
  34 }

 Query description:
   query: "mutation SetFdmLock($fdmLockFromDateList:
    [FdmLockInput!]!) { setFdmLock(fdmLockFromDateList:
    $fdmLockFromDateList) { device { fdmId fdmDateTime
    bufferCapacityUsed } configuration {
    paramsVersionObject { paramsVersion }
    fdmLockFromDateList {fromSystemDate fdmLock lockReason
    { language message } } } } }" , graphQL query
   ($fdmLockFromDateList for input and ConfigurationResult for reply)
     operationName: mutation name
     variables:
         fdmLockFromDate: FdmLockInput array.


Json reply from FDM: (to dataResultUrl endpoint)
 Reply sample:
   1 {
   2     "actionId": "2023-01-05T18:03:23.000Z",
   3     "messageDateTimeEcho": "2024-04-17T04:00:00Z",
   4     "operationType": "SET_FDM_LOCK",
   5     "result": {
   6         "data": {
   7             "setFdmLock": {
   8                 "device": {
   9                     "fdmId": "FOD01000001",
  10                     "fdmDateTime": "2024-04-18T04:00:05Z",
  11                     "bufferCapacityUsed": 35.12,
  12                 },
  13                 "configuration": {
  14                     "paramsVersionObject": {

                                            50
      15                               "paramsVersion": "2023-01-05T18:03:23.000Z"
      16                           },
      17                           "fdmLockFromDateList": [
      18                               {
      19                                   "fromSystemDate": "2024-06-01",
      20                                   "fdmLock": false,
      21                                   "lockReason": null
      22                               },
      23                               {
      24                                   "fromSystemDate": "2024-07-01",
      25                                   "fdmLock": true,
      26                                   "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
      27                                       {"language": "NL", "message": "FDM_LOCKED-NL" },
      28                                       {"language": "FR", "message": "FDM_LOCKED-FR" },
      29                                       {"language": "DE", "message": "FDM_LOCKED-DE" }]
      30                               }
      31                           ]
      32                       }
      33                  }
      34             }
      35        }
      36 }

     Reply description:
       result:
         data:
            ConfigurationResult fields: generic fields for mutations replies - mandatory (see
            12.5.3 Type system)
       If the query fails, the eventual previous values remain valid.


12.5.5.2 Mutation: SetParams

The SetParams query is sent to the FDM to install the latest configuration parameters. The data
of the query contains the latest timestamp version of the parameters ( paramsVersion )
and all current parameters.
The fromSystemDate value(s) indicate when the others values in the object are applied.
There can be many fromSystemDate because it’s an array. When the FDM receives this
array, it applies the values that are in the object with the current fromSystemDate . When
the FDM reaches a fromSystemDate , it applies the values that are in this object.
For example:
   "fromSystemDate": "2024-06-01" → object1
   "fromSystemDate": "2024-07-01" → object2

                                                     51
  The FDM receives this array from the ws_query on “2024-06-10”, so the FDM applies the
  values in object1.
  On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
When the FDM receives this mutation, it installs the new values and drops all the old values. The
new values replace the old ones.
  Json query from ws_query
    GraphQL Type system:
      1   # The fields for paramsVersionObject
      2   input ParamsVersionObjectInput {
      3       paramsVersion: String!    # date time
      4       paramsArray: [ParamsFromSystemDateInput!]
      5   }
      6
      7   input ParamsFromSystemDateInput {
      8       fromSystemDate: String! # YYYY-MM-DD
      9       params: ParamsInput!
     10   }
     11
     12   input ParamsInput {
     13     url: String!
     14     initInterval: Integer!
     15     nopInterval: Integer!
     16     maxBuffer: Integer!
     17     queryInterval: Integer!
     18     retryIntervals: [Integer!]!
     19     retryIntervalsInit: [Integer!]!
     20     maxMsgSize: Integer!
     21     postpone: Boolean!
     22     defaultMaxCalls: Integer!
     23     maxCalls: [MaxCallsInput!]!
     24     footer: [String!]!
     25     vatNo: String
     26     estNo: String
     27     protocolVersion: String!
     28   }
     29
     30   enum WEEKDAY {
     31       MONDAY
     32       TUESDAY
     33       WEDNESDAY
     34       THURSDAY
     35       FRIDAY
     36       SATURDAY
     37       SUNDAY
     38   }
     39
     40   input MaxCallsInput {
     41     dayOfWeek: WEEKDAY!
                                                52
42   hour: Integer! # range 0 - 23.
43   maxCallsInSlot: Integer! # The maximum number of calls allowed in the specified time sl
44 }



Query sample:
 1 {
 2   "dataResultUrl": "https://www.fod/data_endpoint",
 3   "errorResultUrl": "https://www.fod/error_endpoint",
 4   "actionId": "2023-01-05T18:03:23.000Z",
 5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 6   "operationType": "SET_PARAMS",
 7   "executable": {
 8      "query": "mutation SetParams($paramsVersionObject: ParamsVersionObjectInput!)
 9                    { setParams(paramsVersionObject: $paramsVersionObject)
10                        {
11                            device { fdmId fdmDateTime bufferCapacityUsed }
12                            configuration { paramsVersionObject { paramsVersion }
13                            fdmLockFromDateList
14                                {fromSystemDate fdmLock lockReason { language message }
15      "operationName": "SetParams",
16      "variables": {
17        "paramsVersionObject": {
18            "paramsVersion": "2023-01-05T18:03:23.000Z",
19            "paramsArray": [
20                {
21                    "fromSystemDate": "2024-07-01",
22                    "params": {
23                        "url": "https://www.gs2.be/",
24                        "initInterval": 345600,
25                        "nopInterval": 21600,
26                        "maxBuffer": 20000,
27                        "queryInterval": 21600,
28                        "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
29                        "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 10800],
30                        "maxMsgSize": 32,
31                        "postpone": true,
32                        "defaultMaxCalls": 30,
33                        "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "maxCallsInSl
34                            {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot": 100},
35                            {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot": 0}],
36                        "footer": ["myfootertext1", "myfootertext1"],
37                        "vatNo": "1234567890",
38                        "estNo": "0987654321",
39                        "protocolVersion": "0000000001"
40                }
41                },
42                {
43                    "fromSystemDate": "2024-08-01",
44                    "params": {
45                        "url": "https://www.gs2.be/",
46                        "initInterval": 345600,
47                        "nopInterval": 21600,
48                        "maxBuffer": 20000,
49                        "queryInterval": 21600,
50                        "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],

                                         53
  51                          "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 10800],
  52                          "maxMsgSize": 32,
  53                          "postpone": true,
  54                          "defaultMaxCalls": 30,
  55                          "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "maxCallsInSl
  56                              {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot": 100},
  57                              {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot": 0}],
  58                          "footer": ["myfootertext2", "myfootertext2"],
  59                          "vatNo": "1234567890",
  60                          "estNo": "0987654321",
  61                          "protocolVersion": "0000000001"
  62                      }
  63                  }
  64             ]
  65       }
  66     }
  67   }
  68 }

 Query description:
   query: "mutation SetParams($paramsVersionObject:
    ParamsVersionObjectInput!) {
    setParams(paramsVersionObject: $paramsVersionObject) {
    device { fdmId fdmDateTime bufferCapacityUsed }
    configuration { paramsVersionObject { paramsVersion }
    fdmLockFromDateList {fromSystemDate fdmLock lockReason
    { language message } } } } }" , graphQL query
   ($paramsVersionObject for input and ConfigurationResult for reply)
     operationName: mutation name
     variables:
         paramsVersionObject: ParamsVersionObjectInput If this object is
         empty or absent, the FDM must ignore this mutation.


Json reply from FDM: (to dataResultUrl endpoint)
 Reply sample:
   1 {
   2     "actionId": "2023-01-05T18:03:23.000Z",
   3     "messageDateTimeEcho": "2024-04-17T04:00:00Z",
   4     "operationType": "SET_PARAMS",
   5     "result": {
   6         "data": {
   7             "setParams": {
   8                 "device": {
   9                     "fdmId": "FOD01000001",
                                            54
       10                           "fdmDateTime": "2024-04-18T04:00:05Z",
       11                           "bufferCapacityUsed": 35.12,
       12                        },
       13                        "configuration": {
       14                            "paramsVersionObject": {
       15                                "paramsVersion": "2023-01-05T18:03:23.000Z"
       16                            },
       17                            "fdmLockFromDateList": [
       18                                {
       19                                    "fromSystemDate": "2024-06-01",
       20                                    "fdmLock": false,
       21                                    "lockReason": null
       22                                },
       23                                {
       24                                    "fromSystemDate": "2024-07-01",
       25                                    "fdmLock": true,
       26                                    "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
       27                                        {"language": "NL", "message": "FDM_LOCKED-NL" },
       28                                        {"language": "FR", "message": "FDM_LOCKED-FR" },
       29                                        {"language": "DE", "message": "FDM_LOCKED-DE" }]
       30                                }
       31                            ]
       32                        }
       33                  }
       34             }
       35       }
       36 }

      Reply description:
        result:
          data:
             ConfigurationResult fields: generic fields for mutation replies - mandatory (see
             12.5.3 Type system)
       The query was successful and the parameters are updated.
       If the execution of the query was unsuccessful, the eventual previous parameter values
       stored in the FDM remain valid. Warning UPDATE_PARAMS_FAILED should be
       shown on the linked POS systems at the next possible opportunity.
12.5.5.3 Mutation: SetVatRates

This query sends the latest VAT rates to the FDM.
The fromBookingDate value(s) indicate when the other values in the object are applied.
There can be many fromBookingDate because it’s an array. When the FDM receives this
array, it applies the values that are in the object with the current fromBookingDate .
When the FDM reaches a fromBookingDate , it applies the values that are in this object.
For example:
                                                       55
  "fromBookingDate": "2024-06-01" → object1
  "fromBookingDate": "2024-07-01" → object2
  The FDM receives this array from the ws_query on “2024-06-10”, so the FDM applies the
  values in object1.
  On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
When the FDM receives this mutation, it installs the new values and drops all the old values. The
new values replace the old ones.


  Json query from ws_query
    GraphQL Type system:
      1   # +++++ Data in the mutation ++++
      2
      3   # The fields for a VAT rate
      4   input VatRatesInput {
      5      fromBookingDate: String! # YYYY-MM-DD
      6      vatRates: [VatRateInput!]!
      7   }
      8
      9   input VatRateInput {
     10     vatLabel: VatLabel!
     11     vatRate: float!
     12     outOfScope: Boolean!
     13   }
     14
     15   enum VatLabel {
     16     A
     17     B
     18     C
     19     D
     20     X
     21   }
     22




    Query sample:
      1 {
      2   "dataResultUrl": "https://www.fod/data_endpoint",
      3   "errorResultUrl": "https://www.fod/error_endpoint",
      4   "actionId": "2023-01-05T18:03:23.000Z",
      5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
      6   "operationType": "SET_VAT_RATES",
      7   "executable": {
                                               56
 8   "query": "mutation SetVatRates($vatFromBookingDateList: [VatRatesInput!]!)
 9             { setVatRates(vatFromBookingDateList: $vatFromBookingDateList)
10                 {
11                     device { fdmId fdmDateTime bufferCapacityUsed }
12                     configuration { paramsVersionObject { paramsVersion }
13                     fdmLockFromDateList
14                         {fromSystemDate fdmLock lockReason { language message } } }
15   "operationName": "SetVatRates",
16   "variables": {
17      "vatFromBookingDateList": [
18         {
19            "fromBookingDate": "2020-01-01",
20            "vatRates": [
21               {
22                  "vatLabel": "A",
23                  "vatRate": 21,
24                  "outOfScope": false
25               },
26               {
27                  "vatLabel": "B",
28                  "vatRate": 12,
29                  "outOfScope": false
30               },
31               {
32                  "vatLabel": "C",
33                  "vatRate": 6,
34                  "outOfScope": false
35               },
36               {
37                  "vatLabel": "D",
38                  "vatRate": 0,
39                  "outOfScope": false
40               },
41               {
42                  "vatLabel": "X",
43                  "vatRate": 0,
44                  "outOfScope": true
45               }
46            ]
47         },
48         {
49            "fromBookingDate": "2025-04-01",
50            "vatRates": [
51               {
52                  "vatLabel": "A",
53                  "vatRate": 21,
54                  "outOfScope": false
55               },
56               {
57                  "vatLabel": "B",
58                  "vatRate": 17,
59                  "outOfScope": false
60               },
61               {
62                  "vatLabel": "C",
63                  "vatRate": 6,
64                  "outOfScope": false
65               },

                                     57
  66                   {
  67                        "vatLabel": "D",
  68                        "vatRate": 0,
  69                        "outOfScope": false
  70                   },
  71                   {
  72                        "vatLabel": "X",
  73                        "vatRate": 0,
  74                        "outOfScope": true
  75                   }
  76           ]
  77         }
  78       ]
  79     }
  80   }
  81 }

 Query description:
   query: "mutation SetVatRates($vatFromBookingDateList:
    [VatRatesInput!]!) {
    setVatRates(vatFromBookingDateList:
    $vatFromBookingDateList) { device { fdmId fdmDateTime
    bufferCapacityUsed } configuration {
    paramsVersionObject { paramsVersion }
    fdmLockFromDateList {fromSystemDate fdmLock lockReason
    { language message } } } } }" graphQL query
   ($vatFromBookingDateList for input and ConfigurationResult for reply)
     operationName: mutation name
     variables:
       vatFromBookingDateList: VatRatesInput array. If this array is empty or absent, the
       FDM must ignore this mutation,
  The FDM should store the new or modified VAT rates.


Json reply from FDM: (to dataResultUrl endpoint)
 Reply sample:
   1 {
   2     "actionId": "2023-01-05T18:03:23.000Z",
   3     "messageDateTimeEcho": "2024-04-17T04:00:00Z",
   4     "operationType": "SET_VAT_RATES",
   5     "result": {
   6         "data": {
   7             "setVatRates": {
                                             58
        8                       "device": {
        9                           "fdmId": "FOD01000001",
       10                           "fdmDateTime": "2024-04-18T04:00:05Z",
       11                           "bufferCapacityUsed": 35.12,
       12                       },
       13                       "configuration": {
       14                           "paramsVersionObject": {
       15                               "paramsVersion": "2023-01-05T18:03:23.000Z"
       16                           },
       17                           "fdmLockFromDateList": [
       18                               {
       19                                   "fromSystemDate": "2024-06-01",
       20                                   "fdmLock": false,
       21                                   "lockReason": null
       22                               },
       23                               {
       24                                   "fromSystemDate": "2024-07-01",
       25                                   "fdmLock": true,
       26                                   "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
       27                                       {"language": "NL", "message": "FDM_LOCKED-NL" },
       28                                       {"language": "FR", "message": "FDM_LOCKED-FR" },
       29                                       {"language": "DE", "message": "FDM_LOCKED-DE" }]
       30                               }
       31                           ]
       32                       }
       33                  }
       34             }
       35        }
       36 }

      Reply description:
        result:
          data:
             ConfigurationResult fields: generic fields for mutation replies - mandatory (see
             12.5.3 Type system)
       If the execution of the query fails, the eventual previous values in vatList remain
       valid. Warning UPDATE_VAT_RATES_FAILED should be shown on the linked POS
       systems at the next possible opportunity.


12.5.5.4 Mutation: SetPosAllowlist

This query sets the posIdList .
The fromSystemDate value(s) indicate when the other values in the object are applied.
There can be many fromSystemDate because it’s an array. When the FDM receives this
array, it applies the values that are in the object with the current fromSystemDate . When
the FDM reaches a fromSystemDate , it applies the values that are in this object.
                                                      59
Remark : For some FDM (Emergency FDM) used as replacement (during a short time) in case of
FDM breakdown, FPS Finance can decide to allow all POS for a FDM. With these FDM’s, the bad
FDM can be replaced without any configuration (plug and play) before a permanent
replacement. These FDM’s are managed by FPS Finance and producers.
For example:
  "fromSystemDate": "2024-06-01" → object1
  "fromSystemDate": "2024-07-01" → object2
  The FDM receives this array from the ws_query on “2024-06-10”, so the FDM applies the
  values in object1.
  On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
When the FDM receives this mutation, it installs the new values and drops all the old values. The
new values replace the old ones.
  Json query from ws_query
    GraphQL Type system:
     1   # The fields for posIdFromDateList
     2   Input PosAllowlistInput
     3   {
     4       fromSystemDate: String! # YYYY-MM-DD
     5       posIdList: [String]!
     6   }



    Query sample:
      1 {
      2   "dataResultUrl": "https://www.fod/data_endpoint",
      3   "errorResultUrl": "https://www.fod/error_endpoint",
      4   "actionId": "2023-01-05T18:03:23.000Z",
      5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
      6   "operationType": "SET_POS_ALLOWLIST",
      7   "executable": {
      8      "query": "mutation SetPosAllowlist($posIdFromDateList: [PosAllowlistInput!]!)
      9                 { setPosAllowlist (posIdFromDateList: $posIdFromDateList)
     10                     {
     11                         device { fdmId fdmDateTime bufferCapacityUsed }
     12                         configuration { paramsVersionObject { paramsVersion }
     13                         fdmLockFromDateList {fromSystemDate fdmLock lockReason { languag
     14                         posIdFromDateList { fromSystemDate posIdList } } } }",
     15      "operationName": "SetPosAllowlist",
     16      "variables": {
     17         "posIdFromDateList": [
                                                60
  18             {
  19                  "fromSystemDate": "2024-07-01",
  20                  "posIdList": ["Csnpos00000001", "Csnpos00000003"]
  21             },
  22             {
  23                  "fromSystemDate": "2024-08-01",
  24                  "posIdList": ["Csnpos00000001", "Csnpos00000003", "Csnpos00000004"]
  25             }
  26       ]
  27     }
  28   }
  29 }

 Query description:
   query: "mutation SetPosAllowlist($posIdFromDateList:
    [PosAllowlistInput!]!) { setPosAllowlist
    (posIdFromDateList: $posIdFromDateList) {device { fdmId
    fdmDateTime bufferCapacityUsed } configuration {
    paramsVersionObject { paramsVersion }
    fdmLockFromDateList {fromSystemDate fdmLock lockReason
    { language message } } posIdFromDateList {
    fromSystemDate posIdList } } } }" graphQL query
   ($posIdFromDateList for input and ConfigurationResult for reply)
     operationName: mutation name
     variables:
       posIdFromDateList: array of PosAllowlistInput.


Json reply from FDM: (to dataResultUrl endpoint)
 Reply sample:
   1 {
   2     "actionId": "2023-01-05T18:03:23.000Z",
   3     "messageDateTimeEcho": "2024-04-17T04:00:00Z",
   4     "operationType": "SET_POS_ALLOWLIST",
   5     "result": {
   6         "data": {
   7             "setPosAllowlist": {
   8                 "device": {
   9                     "fdmId": "FOD01000001",
  10                     "fdmDateTime": "2024-04-18T04:00:05Z",
  11                     "bufferCapacityUsed": 35.12,
  12                 },
  13                 "configuration": {
  14                     "paramsVersionObject": {
  15                         "paramsVersion": "2023-01-05T18:03:23.000Z"
                                            61
       16                             },
       17                             "fdmLockFromDateList": [
       18                                 {
       19                                     "fromSystemDate": "2024-06-01",
       20                                     "fdmLock": false,
       21                                     "lockReason": null
       22                                 },
       23                                 {
       24                                     "fromSystemDate": "2024-07-01",
       25                                     "fdmLock": true,
       26                                     "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
       27                                         {"language": "NL", "message": "FDM_LOCKED-NL" },
       28                                         {"language": "FR", "message": "FDM_LOCKED-FR" },
       29                                         {"language": "DE", "message": "FDM_LOCKED-DE" }]
       30                                 }
       31                             ],
       32                             "posIdFromDateList": [
       33                                 {
       34                                     "fromSystemDate": "2024-07-01",
       35                                     "posIdList": ["Csnpos00000001", "Csnpos00000003"]
       36                                 },
       37                                 {
       38                                     "fromSystemDate": "2024-08-01",
       39                                     "posIdList": ["Csnpos00000001", "Csnpos00000003", "Csnpos000
       40                                 }
       41                             ]
       42                         }
       43                  }
       44             }
       45        }
       46 }

      Reply description:
        result:
          data:
             ConfigurationResult fields: generic fields for mutation replies - mandatory (see
             12.5.3 Type system)
       If the execution of the query fails, the eventual previous values in
        posIdFromDateList remain valid. Warning
        UPDATE_POS_ALLOWLIST_FAILED should be shown on the linked POS systems
       at next possible opportunity.


12.5.5.5 Mutation: SetTrustCert

This query provides the chain of trust certificates which allow the FDM to resolve the server
certificate from FPS Finance.


                                                        62
The fromSystemDate value(s) indicate when the other values in the object are applied.
There can be many fromSystemDate because it’s an array. When the FDM receives this
array, it applies the values that are in the object with the current fromSystemDate . When
the FDM reaches a fromSystemDate , it applies the values that are in this object.
For example:
  "fromSystemDate": "2024-06-01" → object1
  "fromSystemDate": "2024-07-01" → object2
  The FDM receive this array from the ws_query on “2024-06-10”, so the FDM applies the
  values in object1.
  On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
When the FDM receives this mutation, it installs the new values and drops all the old values. The
new values replace the old ones.
  Json query from ws_query
    GraphQL Type system:
     1   # +++++ Data in the mutation ++++
     2
     3   # The fields for TrustCertInput
     4   input TrustCertInput {
     5       fromSystemDate: String! # YYYY-MM-DD
     6       trustCert: [String!]!
     7   }

    Query sample:
      1 {
      2   "dataResultUrl": "https://www.fod/data_endpoint",
      3   "errorResultUrl": "https://www.fod/error_endpoint",
      4   "actionId": "2023-01-05T18:03:23.000Z",
      5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
      6   "operationType": "SET_TRUST_CERT",
      7   "executable": {
      8      "query": "mutation SetTrustCert($trustCertFromDateList: [TrustCertInput!]!)
      9                    { setTrustCert(trustCertFromDateList: $trustCertFromDateList)
     10                        {
     11                            device { fdmId fdmDateTime bufferCapacityUsed }
     12                            configuration { paramsVersionObject { paramsVersion }
     13                            fdmLockFromDateList
     14                                {fromSystemDate fdmLock lockReason { language message }
     15      "operationName": "SetTrustCert",
     16      "variables": {

                                                63
  17       "trustCertFromDateList": [
  18           {
  19               "fromSystemDate": "2024-07-01",
  20               "trustCert": [ "cert1x509pem", "cert2x509pem", "cert3x509pem" ]
  21           },
  22           {
  23               "fromSystemDate": "2024-08-01",
  24               "trustCert": [ "cert1x509pem2", "cert2x509pem2", "cert3x509pem2" ]
  25           }
  26       ]
  27     }
  28   }
  29 }

 Query description:
   query: "mutation SetTrustCert($trustCertFromDateList:
    [TrustCertInput!]!) {
    setTrustCert(trustCertFromDateList:
    $trustCertFromDateList) { device { fdmId fdmDateTime
    bufferCapacityUsed } configuration {
    paramsVersionObject { paramsVersion }
    fdmLockFromDateList {fromSystemDate fdmLock lockReason
    { language message } } } } }" graphQL query
   ($trustCertFromDateList for input and ConfigurationResult for reply)
     operationName: mutation name
     variables:
         trustCertFromDateList: array of TrustCertInput . If this array is empty or
         absent, the FDM must ignore this mutation,
  The query trustCert contains a list of X.509 certificates in PEM format. The
  certificates should be stored in the FDM's certificate store certstoreTrust . The
  FDM may delete all other trust certificates from the certificate store
   certstoreTrust .


Json reply from FDM: (to dataResultUrl endpoint)
 Reply sample:
   1 {
   2      "actionId": "2023-01-05T18:03:23.000Z",
   3      "messageDateTimeEcho": "2024-04-17T04:00:00Z",
   4      "operationType": "SET_TRUST_CERT",
                                            64
        5        "result": {
        6            "data": {
        7                "setTrustCert": {
        8                    "device": {
        9                        "fdmId": "FOD01000001",
       10                        "fdmDateTime": "2024-04-18T04:00:05Z",
       11                        "bufferCapacityUsed": 35.12,
       12                    },
       13                    "configuration": {
       14                        "paramsVersionObject": {
       15                            "paramsVersion": "2023-01-05T18:03:23.000Z"
       16                        },
       17                        "fdmLockFromDateList": [
       18                            {
       19                                "fromSystemDate": "2024-06-01",
       20                                "fdmLock": false,
       21                                "lockReason": null
       22                            },
       23                            {
       24                                "fromSystemDate": "2024-07-01",
       25                                "fdmLock": true,
       26                                "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
       27                                    {"language": "NL", "message": "FDM_LOCKED-NL" },
       28                                    {"language": "FR", "message": "FDM_LOCKED-FR" },
       29                                    {"language": "DE", "message": "FDM_LOCKED-DE" }]
       30                            }
       31                        ]
       32                    }
       33                }
       34            }
       35        }
       36 }

      Reply description:
        result:
          data:
             ConfigurationResult fields: generic fields for mutation replies - mandatory (see
             12.5.3 Type system)
       If the execution of the query fails, the eventual previous certstoreTrust remains
       unchanged. Warning UPDATE_TRUST_CERT_FAILED should be shown on the
       linked POS systems at next possible opportunity.


12.5.5.6 Mutation: SetUrls

The SetUrls query is used to install the latest URLs of the different webservices.
The fromSystemDate value(s) indicate when the others values in the object are applied.
There can be many fromSystemDate because it’s an array. When the FDM receives this

                                                   65
array, it applies the values that are in the object with the current fromSystemDate . When
the FDM reaches a fromSystemDate , it applies the values that are in this object.
For example:
  "fromSystemDate": "2024-06-01" → object1
  "fromSystemDate": "2024-07-01" → object2
  The FDM receives this array from the ws_query on “2024-06-10”, so the FDM applies the
  values in object1.
  On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
When the FDM receives this mutation, it installs the new values and drops all the old values. The
new values replace the old ones.
  Json query from ws_query
    GraphQL Type system:
      1   # The fields for a URLs values
      2   input UrlInput {
      3       fromSystemDate: String! # YYYY-MM-DD
      4       urls: [KeyValueInput!]!
      5   }
      6
      7   input KeyValueInput {
      8      key: String!
      9      value: String!
     10   }



    Query sample:
      1 {
      2   "dataResultUrl": "https://www.fod/data_endpoint",
      3   "errorResultUrl": "https://www.fod/error_endpoint",
      4   "actionId": "2023-01-05T18:03:23.000Z",
      5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
      6   "operationType": "SET_URLS",
      7   "executable": {
      8      "query": "mutation SetUrls($urlsFromDateList: [UrlInput!]!)
      9                    { setUrls(urlsFromDateList: $urlsFromDateList)
     10                        {
     11                            device { fdmId fdmDateTime bufferCapacityUsed }
     12                            configuration { paramsVersionObject { paramsVersion }
     13                            fdmLockFromDateList
     14                                {fromSystemDate fdmLock lockReason { language message }
     15      "operationName": "SetUrls",

                                                66
  16     "variables": {
  17       "urlsFromDateList": [
  18           {
  19               "fromSystemDate": "2024-07-01",
  20               "urls": [
  21                   { "key": "ws_reg", "value": "https://myurl:8080" },
  22                   { "key": "ws_nop", "value": "https://myurl:8080" },
  23                   { "key": "ws_query", "value": "https://myurl:8080" },
  24                   { "key": "ws_init", "value": "https://myurl:8080" },
  25                   { "key": "ws_ntp1", "value": "https://myurl:8080" },
  26                   { "key": "ws_ntp2", "value": "https://myurl:8080" }
  27               ]
  28           },
  29           {
  30               "fromSystemDate": "2024-08-01",
  31               "urls": [
  32                   { "key": "ws_reg", "value": "https://myurl2:8080" },
  33                   { "key": "ws_nop", "value": "https://myurl2:8080" },
  34                   { "key": "ws_query", "value": "https://myurl2:8080" },
  35                   { "key": "ws_init", "value": "https://myurl2:8080" },
  36                   { "key": "ws_ntp1", "value": "https://myurl2:8080" },
  37                   { "key": "ws_ntp2", "value": "https://myurl2:8080" }
  38               ]
  39           }
  40       ]
  41     }
  42   }
  43 }

 Query description:
   query: "mutation SetUrls($urlsFromDateList: [UrlInput!]!)
    { setUrls(urlsFromDateList: $urlsFromDateList) { device
    { fdmId fdmDateTime bufferCapacityUsed } configuration
    { paramsVersionObject { paramsVersion }
    fdmLockFromDateList {fromSystemDate fdmLock lockReason
    { language message } } } } }" graphQL query ($urlsFromDateList for
   input and ConfigurationResult for reply)
     operationName: mutation name
     variables:
        urlsFromDateList: UrlInput array. If this array is empty or absent, the FDM must
        ignore this mutation,


Json reply from FDM: (to dataResultUrl endpoint)
 Reply sample:
   1 {
                                           67
 2      "actionId": "2023-01-05T18:03:23.000Z",
 3      "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 4      "operationType": "SET_URLS",
 5      "result": {
 6          "data": {
 7              "setUrls": {
 8                  "device": {
 9                      "fdmId": "FOD01000001",
10                      "fdmDateTime": "2024-04-18T04:00:05Z",
11                      "bufferCapacityUsed": 35.12,
12                  },
13                  "configuration": {
14                      "paramsVersionObject": {
15                          "paramsVersion": "2023-01-05T18:03:23.000Z"
16                      },
17                      "fdmLockFromDateList": [
18                          {
19                              "fromSystemDate": "2024-06-01",
20                              "fdmLock": false,
21                              "lockReason": null
22                          },
23                          {
24                              "fromSystemDate": "2024-07-01",
25                              "fdmLock": true,
26                              "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
27                                  {"language": "NL", "message": "FDM_LOCKED-NL" },
28                                  {"language": "FR", "message": "FDM_LOCKED-FR" },
29                                  {"language": "DE", "message": "FDM_LOCKED-DE" }]
30                          }
31                      ]
32                  }
33              }
34          }
35      }
36 }

Reply description:
  result:
    data:
       ConfigurationResult fields: generic fields for mutations replies - mandatory (see
       12.5.3 Type system)
 All other replies: the update failed. The eventual previous values remain valid. Warning
  UPDATE_URLS_FAILED should be shown on the linked POS systems at the next
 possible opportunity.
 If the execution of the query fails, the eventual previous values remain unchanged.
 Warning UPDATE_URLS_FAILED should be shown on the linked POS systems at the
 next possible opportunity.


                                           68
12.5.5.7 Mutation: SetClientCert

The SetClientCert query is used to renew an existing operational client certificate. The
renewal involves 2 steps: recollection (in the query) and validation of the new client certificate
(with the reply).
The fromSystemDate value(s) indicate when the other values in the object are applied.
There can be many fromSystemDate because it’s an array. When the FDM receives this
array, it applies the values that are in the object with the current fromSystemDate . When
the FDM reaches a fromSystemDate , it applies the values that are in this object.
For example:
   "fromSystemDate": "2024-06-01" → object1
   "fromSystemDate": "2024-07-01" → object2
   The FDM receives this array from the ws_query on “2024-06-10”, so the FDM applies the
   values in object1.
   On "2024-07-01", the FDM applies object2.
Remark: When the FDM receives the mutation, the array must contain a value for the current
date.
When the FDM receives this mutation, it installs the new values (after step 2, with positive reply
recieved) and drops all the old values. The new values replace the old ones.
   Json query from ws_query
     GraphQL Type system:
       1   # +++++ Data in the mutation ++++
       2
       3   # The fields for ClientCertInput
       4   input ClientCertInput {
       5       fromSystemDate: String! # YYYY-MM-DD
       6       clientCert: String!
       7   }

      Query sample:
        1 {
        2   "dataResultUrl": "https://www.fod/data_endpoint",
        3   "errorResultUrl": "https://www.fod/error_endpoint",
        4   "actionId": "2023-01-05T18:03:23.000Z",
        5   "messageDateTimeEcho": "2024-04-17T04:00:00Z",
        6   "operationType": "SET_CLIENT_CERT",
        7   "executable": {
        8      "query": "mutation SetClientCert($clientCertFromDateList: [ClientCertInput!]!)
        9                    { setClientCert(clientCertFromDateList: $clientCertFromDateList)
                                                  69
   10                       {
   11                           device { fdmId fdmDateTime bufferCapacityUsed }
   12                           configuration { paramsVersionObject { paramsVersion }
   13                           fdmLockFromDateList
   14                               {fromSystemDate fdmLock lockReason { language message }
   15                           clientCertFromDateList { fromSystemDate clientCertSn } } } }
   16     "operationName": "SetClientCert",
   17     "variables": {
   18       "clientCertFromDateList": [
   19           {
   20               "fromSystemDate": "2024-07-01",
   21               "clientCert": "pkcs12pem-with-password"
   22           },
   23           {
   24               "fromSystemDate": "2024-08-01",
   25               "clientCert": "pkcs12pem2-with-password"
   26           }
   27       ]
   28     }
   29   }
   30 }

Query description:
  query: "mutation SetClientCert($clientCertFromDateList:
   [ClientCertInput!]!) {
   setClientCert(clientCertFromDateList:
   $clientCertFromDateList) { device { fdmId fdmDateTime
   bufferCapacityUsed } configuration { paramsVersionObject
   { paramsVersion } fdmLockFromDateList {fromSystemDate
   fdmLock lockReason { language message } }
   clientCertFromDateList { fromSystemDate clientCertSn } }
   } }" graphQL query ($clientCertFromDateList for input and ConfigurationResult for
  reply)
    operationName: mutation name
    variables:
      clientCertFromDateList: clientCertFromDateList array with object
       clientCert containing the client certificate and the private key (encryption
      password = secret, details can be requested from the relevant department). If this
      array is empty or absent, the FDM must ignore this mutation.


Json reply from FDM: (to dataResultUrl endpoint)

                                           70
Step 1: The FDM can import the candidate client certificate(s) and private key(s) using the
shared secret and FDM serial number. The FDM should however not remove its operational
client certificate.
Step 2: In this step, the FDM performs a confirmation call with the candidate (depending on
the right fromSystemDate with the current date) client certificate from the previous
step.
   Protocol: mTLS using candidate client certificate from the previous step
   Verb: GET
  URL: "dataResultUrl"
  Possible outcomes are:
    The FDM receives a positive reply (HTTP code 200) → The FDM should promote the
    candidate client certificate to the new operational client certificate.
    The FDM receives a negative reply (HTTP code 4XX or 5XX) → The confirmation
    failed. The FDM can’t promote the client certificate. The renewal procedure is a failure.
Reply sample:
 1 {
 2      "actionId": "2023-01-05T18:03:23.000Z",
 3      "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 4      "operationType": "SET_CLIENT_CERT",
 5      "result": {
 6          "data": {
 7              "setClientCert": {
 8                  "device": {
 9                      "fdmId": "FOD01000001",
10                      "fdmDateTime": "2024-04-18T04:00:05Z",
11                      "bufferCapacityUsed": 35.12,
12                  },
13                  "configuration": {
14                      "paramsVersionObject": {
15                          "paramsVersion": "2023-01-05T18:03:23.000Z"
16                      },
17                      "fdmLockFromDateList": [
18                          {
19                              "fromSystemDate": "2024-06-01",
20                              "fdmLock": false,
21                              "lockReason": null
22                          },
23                          {
24                              "fromSystemDate": "2024-07-01",
25                              "fdmLock": true,
26                              "lockReason": [{"language": "EN", "message": "FDM_LOCKED-EN"
27                                  {"language": "NL", "message": "FDM_LOCKED-NL" },
28                                  {"language": "FR", "message": "FDM_LOCKED-FR" },
29                                  {"language": "DE", "message": "FDM_LOCKED-DE" }]
30                          }
31                      ],
32                      "clientCertFromDateList": [
                                           71
       33                                {
       34                                     "fromSystemDate": "2024-07-01",
       35                                     "clientCertSn": "dhjs76djs"
       36                                },
       37                                {
       38                                     "fromSystemDate": "2024-08-01",
       39                                     "clientCertSn": "dhjs76djs2"
       40                                }
       41                            ]
       42                        }
       43                 }
       44            }
       45       }
       46 }

      Reply description:
        result:
          data:
             ConfigurationResult fields: generic fields for mutation replies - mandatory (see
             12.5.3 Type system)
                clientCertSn: serial number of the candidate client certificate
       If the query was unsuccessful, warning UPDATE_CLIENT_CERT_FAILED should
       be shown on the linked POS systems at next possible opportunity.


12.5.5.8 Query: FailedRequests

This query requests the failed requests error log (see 12.2 Error logging).
   Json query from ws_query
     GraphQL Type system:
       1 #### Input fields for Failed Requests
       2
       3 input DateTimeRangeInput {
       4   from: String!            # "2024-04-09T04:00:00Z" for example
       5   to: String!              # "2024-04-09T04:00:00Z" for example
       6   maxItemCount: Integer!   # maximum items that will be send
       7                            # (last maxItemCount for the range)
       8 }
       9

      Query sample:
        1 {
        2       "dataResultUrl": "https://www.fod/data_endpoint",
        3       "errorResultUrl": "https://www.fod/error_endpoint",
        4       "actionId": "2023-01-05T18:03:23.000Z",
        5       "messageDateTimeEcho": "2024-04-17T04:00:00Z",
        6       "operationType": "FAILED_REQUESTS",
        7       "executable": {
                                                        72
        8          "query": "query FailedRequests($selectRange: DateTimeRangeInput!)
        9                      { failedRequests(selectRange: $selectRange)
       10                          { timestamp requestMethod requestUri requestHeaders
       11                          requestBody replyStatusCode replyHeaders replyBody errorMessage
       12          "operationName": "FailedRequests",
       13          "variables": {
       14              "selectRange": {
       15                  "from": "2024-04-17T00:00:00Z",
       16                  "to": "2024-04-18T00:00:00Z",
       17                  "maxItemCount": 30
       18              }
       19          }
       20      }
       21 }

 Query description:
   query: "query FailedRequests($selectRange:
       DateTimeRangeInput!) { failedRequests(selectRange:
       $selectRange) { timestamp requestMethod requestUri
       requestHeaders requestBody replyStatusCode replyHeaders
       replyBody errorMessage } }" graphQL query ($selectRange for input and
   [FailedRequest] for reply)
     operationName: query name
     variables:
        selectRange: selectRange object
              Remark: the result is limited to the last maxItemCount items in the range
 Json reply from FDM: (to dataResultUrl endpoint)
   Reply samples:
 1 {
 2       "actionId": "2023-01-05T18:03:23.000Z",
 3       "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 4       "operationType": "FAILED_REQUESTS",
 5       "result": {
 6           "data": {
 7               "failedRequests": [
 8                   {
 9                       "timestamp": "2024-04-18T04:00:05Z",
10                       "requestMethod": "request Method",
11                       "requestUri": "request Uri",
12                       "requestHeaders": "request Headers",
13                       "requestBody": "request Body",
14                       "replyStatusCode": 500,
15                       "replyHeaders": "reply Headers",
16                       "replyBody": "reply Body",
17                       "errorMessage": "error"
18                   },
19                   {
                                                 73
 20                           "timestamp": "2024-04-20T04:00:05Z",
 21                           "requestMethod": "request Method",
 22                           "requestUri": "request Uri",
 23                           "requestHeaders": "request Headers",
 24                           "requestBody": "request Body",
 25                           "replyStatusCode": 500,
 26                           "replyHeaders": "reply Headers",
 27                           "replyBody": "reply Body",
 28                           "errorMessage": "error"
 29                      }
 30                 ]
 31            }
 32       }
 33 }

No error logging result:
  1 {
  2       "actionId": "2023-01-05T18:03:23.000Z",
  3       "messageDateTimeEcho": "2024-04-17T04:00:00Z",
  4       "operationType": "FAILED_REQUESTS",
  5       "result": {
  6           "data": {
  7               "failedRequests": []
  8           }
  9       }
 10 }




   Reply description
     result:
       data:
          [FailedRequests] array: - mandatory


12.5.5.9 Query: AllFdmInfos

This query requests all the FDM configurations/info.
   Json query from ws_query
     Query sample:
  1 {
  2       "dataResultUrl": "https://www.fod/data_endpoint",
  3       "errorResultUrl": "https://www.fod/error_endpoint",
  4       "actionId": "2023-01-05T18:03:23.000Z",
  5       "messageDateTimeEcho": "2024-04-17T04:00:00Z",
  6       "operationType": "ALL_FDM_INFOS",
  7       "executable": {
  8           "query": "query AllFdmInfos { allFdmInfos
  9               {
 10                   device { fdmId fdmSwVersion fdmDateTime bufferCapacityUsed }
 11                   transactionState { unsentEventCount fdmLastSuccessfulTrans retryCount }
                                                     74
12                   taskState { lastDownload }
13                   configuration {
14                       fdmLockFromDateList { fromSystemDate fdmLock lockReason { language messag
15                       paramsVersionObject { paramsVersion paramsArray { fromSystemDate params
16                           { url initInterval nopInterval maxBuffer queryInterval retryIntervals
17                       maxMsgSize postpone defaultMaxCalls maxCalls { dayOfWeek hour maxCallsInS
18                       vatFromBookingDateList { fromBookingDate vatRates { vatLabel vatRate outO
19                       posIdFromDateList { fromSystemDate posIdList }
20                       trustCertFromDateList { fromSystemDate trustCert }
21                       urlsFromDateList { fromSystemDate urls { key value } }
22                       clientCertFromDateList { fromSystemDate clientCertSn }
23                       localConfig { ws_ntp3 }
24                   } } }",
25           "operationName": "AllFdmInfos"
26       }
27 }

 Query description:
   query: device { fdmId fdmSwVersion fdmDateTime
       bufferCapacityUsed } transactionState { unsentEventCount
       fdmLastSuccessfulTrans retryCount } taskState {
       lastDownload } configuration { fdmLockFromDateList {
       fromSystemDate fdmLock lockReason { language message } }
       paramsVersionObject { paramsVersion paramsArray {
       fromSystemDate params { url initInterval nopInterval
       maxBuffer queryInterval retryIntervals retryIntervalsInit
       maxMsgSize postpone defaultMaxCalls maxCalls { dayOfWeek
       hour maxCallsInSlot} footer vatNo estNo protocolVersion }
       } } vatFromBookingDateList { fromBookingDate vatRates {
       vatLabel vatRate outOfScope } } posIdFromDateList {
       fromSystemDate posIdList } trustCertFromDateList {
       fromSystemDate trustCert } urlsFromDateList {
       fromSystemDate urls { key value } }
       clientCertFromDateList { fromSystemDate clientCertSn }
       localConfig { ws_ntp3 } } } graphQL query (AllFdmInfos for reply)
        operationName: query name
        variables:



                                                75
Json reply from FDM: (to dataResultUrl endpoint)
  Reply samples:
 1 {
 2     "actionId": "2023-01-05T18:03:23.000Z",
 3     "messageDateTimeEcho": "2024-04-17T04:00:00Z",
 4     "operationType": "ALL_FDM_INFOS",
 5     "result": {
 6         "data": {
 7             "allFdmInfos": {
 8                 "device": {
 9                     "fdmId": "FOD01000001",
10                     "fdmSwVersion": "1.2.0",
11                     "fdmDateTime": "2022-10-10T10:00:39",
12                     "bufferCapacityUsed": 10.02
13                 },
14                 "transactionState": {
15                     "unsentEventCount": 0,
16                     "fdmLastSuccessfulTrans": "2022-10-09T10:00:39",
17                     "retryCount": 0
18                 },
19                 "taskState": {
20                     "lastDownload": "2022-10-08T10:00:39"
21                 },
22                 "configuration": {
23                     "fdmLockFromDateList": [
24                         {
25                             "fromSystemDate": "2024-06-01",
26                             "fdmLock": false,
27                             "lockReason": null
28                         }
29                     ],
30                     "paramsVersionObject": {
31                         "paramsVersion": "2023-01-05T18:03:23.000Z",
32                         "paramsArray": [
33                             {
34                                 "fromSystemDate": "2024-07-01",
35                                 "params": {
36                                     "url": "https://www.gs2.be/",
37                                     "initInterval": 345600,
38                                     "nopInterval": 21600,
39                                     "maxBuffer": 20000,
40                                     "queryInterval": 21600,
41                                     "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
42                                     "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 10800
43                                     "maxMsgSize": 32,
44                                     "postpone": true,
45                                     "defaultMaxCalls": 30,
46                                     "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "maxC
47                                         {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot"
48                                         {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot"
49                                     "footer": ["myfootertext1", "myfootertext1"],
50                                     "vatNo": "1234567890",
51                                     "estNo": "0987654321",
52                                     "protocolVersion": "0000000001"
53                                 }
54                             },
                                             76
 55          {
 56              "fromSystemDate": "2024-08-01",
 57              "params": {
 58                  "url": "https://www.gs2.be/",
 59                  "initInterval": 345600,
 60                  "nopInterval": 21600,
 61                  "maxBuffer": 20000,
 62                  "queryInterval": 21600,
 63                  "retryIntervals": [5, 60, 300, 300, 300, 3600, 10800],
 64                  "retryIntervalsInit": [5, 60, 300, 300, 300, 3600, 10800
 65                  "maxMsgSize": 32,
 66                  "postpone": true,
 67                  "defaultMaxCalls": 30,
 68                  "maxCalls": [{"dayOfWeek": "SATURDAY", "hour": 18, "maxC
 69                      {"dayOfWeek": "SUNDAY", "hour": 12, "maxCallsInSlot"
 70                      {"dayOfWeek": "MONDAY", "hour": 17, "maxCallsInSlot"
 71                  "footer": ["myfootertext2", "myfootertext2"],
 72                  "vatNo": "1234567890",
 73                  "estNo": "0987654321",
 74                  "protocolVersion": "0000000001"
 75              }
 76          }
 77      ]
 78   }
 79   "vatFromBookingDateList":
 80   [
 81       {
 82           "fromBookingDate": "2020-01-01",
 83           "vatRates": [
 84               {
 85                   "vatLabel": "A",
 86                   "vatRate": 21,
 87                   "outOfScope": false
 88               },
 89               {
 90                   "vatLabel": "B",
 91                   "vatRate": 12,
 92                   "outOfScope": false
 93               },
 94               {
 95                   "vatLabel": "C",
 96                   "vatRate": 6,
 97                   "outOfScope": false
 98               },
 99               {
100                   "vatLabel": "D",
101                   "vatRate": 0,
102                   "outOfScope": false
103               },
104               {
105                   "vatLabel": "X",
106                   "vatRate": 0,
107                   "outOfScope": true
108               }
109           ]
110       },
111       {
112           "fromBookingDate": "2025-04-01",

                            77
113          "vatRates": [
114              {
115                  "vatLabel": "A",
116                  "vatRate": 21,
117                  "outOfScope": false
118              },
119              {
120                  "vatLabel": "B",
121                  "vatRate": 17,
122                  "outOfScope": false
123              },
124              {
125                  "vatLabel": "C",
126                  "vatRate": 6,
127                  "outOfScope": false
128              },
129              {
130                  "vatLabel": "D",
131                  "vatRate": 0,
132                  "outOfScope": false
133              },
134              {
135                  "vatLabel": "X",
136                  "vatRate": 0,
137                  "outOfScope": true
138              }
139          ]
140       }
141   ],
142   "posIdFromDateList": [
143       {
144           "fromSystemDate": "2024-07-01",
145           "posIdList": ["Csnpos00000001", "Csnpos00000003"]
146       },
147       {
148           "fromSystemDate": "2024-08-01",
149           "posIdList": ["Csnpos00000001", "Csnpos00000003", "Csnpos0000000
150       }
151   ],
152   "trustCertFromDateList": [
153       {
154           "fromSystemDate": "2024-07-01",
155           "trustCert": [ "cert1x509pem", "cert2x509pem", "cert3x509pem" ]
156       },
157       {
158           "fromSystemDate": "2024-08-01",
159           "trustCert": [ "cert1x509pem2", "cert2x509pem2", "cert3x509pem2"
160       }
161   ],
162   "urlsFromDateList": [
163       {
164           "fromSystemDate": "2024-07-01",
165           "urls": [
166               { "key": "ws_reg", "value": "https://myurl:8080" },
167               { "key": "ws_nop", "value": "https://myurl:8080" },
168               { "key": "ws_query", "value": "https://myurl:8080" },
169               { "key": "ws_init", "value": "https://myurl:8080" },
170               { "key": "ws_ntp1", "value": "https://myurl:8080" },

                            78
171                                   { "key": "ws_ntp2", "value": "https://myurl:8080" }
172                               ]
173                          },
174                          {
175                               "fromSystemDate": "2024-08-01",
176                               "urls": [
177                                   { "key": "ws_reg", "value": "https://myurl2:8080" },
178                                   { "key": "ws_nop", "value": "https://myurl2:8080" },
179                                   { "key": "ws_query", "value": "https://myurl2:8080" },
180                                   { "key": "ws_init", "value": "https://myurl2:8080" },
181                                   { "key": "ws_ntp1", "value": "https://myurl2:8080" },
182                                   { "key": "ws_ntp2", "value": "https://myurl2:8080" }
183                               ]
184                          }
185                      ],
186                      "clientCertFromDateList": [
187                          {
188                              "fromSystemDate": "2024-07-01",
189                              "clientCertSn": "dhjs76djs"
190                          },
191                          {
192                              "fromSystemDate": "2024-08-01",
193                              "clientCertSn": "dhjs76djs2"
194                          }
195                      ],
196                      "localConfig": {
197                          "ws_ntp3": "https://myurl8:8080"
198                      }
199                  }
200              }
201          }
202      }
203 }




  Reply description:
    result:
      data:
         AllFdmInfos



13. Warnings and errors summary
Warnings and errors are sent to the connected POS systems at the next possible opportunity for
each incident. The POS system is responsible for handling the warnings and errors and
translating them into useful output for the user(s).
Warnings are incidents that can escalate over time into an error. An error means that the FDM
will refuse to accept and sign the business transaction.

                                                79
Cod Code            T           Message                           Remedy
e                   y
(old)               p
                    e
W00 CLIENT_CER w Validity of client certificate   The FDM should call the query
1   T_NEAR_EXP ar is less than 4 months.          webservice to receive the query
    IRATION    ni                                 SetClientCert.
               ng
W00 CLIENT_CER w Client certificate has           Check the e-service of FPS Finance if
2   T_EXPIRED ar expired.                         your FDM unit is eligible for a client
               ni                                 certificate. FDM should automatically
               ng                                 reinitialize itself. In case of issues,
                                                  contact your dealer.
W00 RTC_SYNC_F w FDM could not synchronize Possible causes:
3   AILED      ar real-time clock.           issue with internet connection;
               ni                            url of time server is outdated;
               ng
                                             unavailability of time server.
                                                  The user can manually try to resync
                                                  the real-time clock with the NTP
                                                  timeserver.
                                                  The user can manually try to call the
                                                  query webservice (ws_query).
                                                  The user can manually try to update
                                                  the URLs or adapt the URL of the time
                                                  server manually via the Administration
                                                  Console.
                                                  If the issue, related to an outdated
                                                  URL, persists, contact your dealer.
W00 UPDATE_URL w FDM could not update             Possible causes:
4   S_FAILED   ar URLs.                             Internal problem
               ni
                                                  Contact your dealer.
               ng


                                          80
W00 UPDATE_TRU w The fiscal data module              Possible causes:
5   ST_CERT_FAI ar could not update the trust          Internal problem
    LED         ni certificates for the
                                                     Contact your dealer.
                ng webservices.
W00 UPDATE_TAS w The fiscal data module           Possible causes:
6   K_LIST_FAIL ar could not obtain its task list   issue with internet connection, client
    ED          ni from the webservice.             certificate or trust store;
                ng                                  unavailability of online service.
                                                     The user can manually try to call query
                                                     webservice (ws_query).
                                                     The FDM should try to reinitialize itself
                                                     automatically if the issue, related to
                                                     client certificate, trust store or
                                                     outdated URLs, persists. You might
                                                     need to manually update the init URL
                                                     via the Administration Console if it is
                                                     outdated. If auto-reinitialization does
                                                     not work, contact your dealer.
W00 TASK_FEEDB w The fiscal data module        Possible causes:
7   ACK_FAILED ar could not report feedback      issue with internet connection, client
               ni on a task to the webservice.   certificate or trust store;
               ng                                unavailability of online service.
                                                     The user can manually try to call query
                                                     webservice (ws_query).
                                                     The FDM should try to reinitialize itself
                                                     automatically if the issue, related to
                                                     client certificate, trust store or
                                                     outdated URLs, persists. You might
                                                     need to manually update the init URL
                                                     via the Administration Console if it is
                                                     outdated. If auto-reinitialization does
                                                     not work, contact your dealer.



                                            81
W00 NOP_FAILED w The fiscal data module        Possible causes:
8              ar could not post a no            issue with internet connection, client
               ni operation message to the       certificate or trust store;
               ng webservice.                    unavailability of online service.
                                               The user can manually try to call query
                                               webservice (ws_query).
                                               FDM should try to reinitialize itself
                                               automatically if the issue, related to
                                               client certificate, trust store or
                                               outdated URLs, persists. You might
                                               need to manually update the init URL
                                               via the Administration Console if it is
                                               outdated. If auto-reinitialization does
                                               not work, contact your dealer.
W00 BUFFER_NEA w The buffer usage exceeds      Possible causes:
9   R_FULL     ar 70%.                           issue with internet connection, client
               ni                                certificate or trust store;
               ng                                unavailability of online service.
                                               The user can consult the last date and
                                               time of successful transmission, and
                                               the error log.
                                               FDM should try to reinitialize itself
                                               automatically if the issue, related to
                                               client certificate, trust store or
                                               outdated URLs, persists. You might
                                               need to manually update the init URL
                                               via the Administration Console if it is
                                               outdated. If auto-reinitialization does
                                               not work, contact your dealer.
W01 SERVER_CER w The fiscal data module        The user can manually try to call query
0   T_RESOLVE_ ar could not resolve a server   webservice (ws_query). If the issue
    FAILED     ni certificate.                 ( SERVER_CERT_RESOLVE_FAI
               ng                              LED ) persists, the FDM should set

                                       82
                                                fdmInit = true to perform the
                                                initialization webservice (ws_init) at the
                                                next boot.
W011 TRANSACTI w The fiscal data module         The user should troubleshoot the
     ON_UPLOAD_ ar could not post a business    internet connection or the webservices
     FAILED     ni transaction to the           if they are not not available.
                ng webservice.                  Furthermore, via de e-service of FPS
                       The FDM puts the         Finance, you can check if your FDM is
                                                properly registered and activated.
                        retryCount in the
                       message.                 Possible causes:
                                                  issue with internet connection, client
                                                  certificate or trust store;
                                                  URLs are outdated;
                                                  unavailability of online service.
                                                The user can manually try to call query
                                                webservice (ws_query).
                                                FDM should try to reinitialize itself
                                                automatically if the issue, related to
                                                client certificate, trust store or
                                                outdated URLs, persists. You might
                                                need to manually update the init URL
                                                via the Administration Console if it is
                                                outdated. If auto-reinitialization does
                                                not work, contact your dealer.
W01 INITIALIZAT w Initialization failed.        Check internet connection. Check URL
2   ION_FAILED ar                               of initialization webservice. Check
                ni                              date/time settings. Check e-service of
                ng                              FPS Finance if your device is eligible
                                                for online services.
W01 RTC_NOT_IN w Check FDM battery.             Check FDM battery.
3   ITIALIZED  ar                               If the problem persist, possible
               ni                               causes:
               ng
                                                  issue with internet connection;
                                                  URLs are outdated;
                                           83
                                                  unavailability of online service
                                                  ws_ntp1, ws_ntp2 and ws_ntp3 if
                                                  specified.

W01 CORRUPT_R w Corrupt transaction in          Check FDM memory.
4   ECORD_ENC ar memory                         If the problem persist, contact FDM
    OUNTERED ni                                 Dealer/Manufacturer
              ng
W01 UPDATE_PAR w FDM could not update           Possible causes:
5   AMS_FAILED ar params.                         Internal problem
               ni
                                                Contact your dealer.
               ng
W01 UPDATE_CLI w FDM could not update client Possible causes:
6   ENT_CERT_F ar certificate.                 Internal problem
    AILED      ni
                                             Contact your dealer.
               ng
W01 UPDATE_VAT w FDM could not update VAT       Possible causes:
7   _RATES_FAIL ar rates.                         Internal problem
    ED          ni
                                                Contact your dealer.
                ng
W01 UDPATE_POS w The fiscal data module         Possible causes:
8   _ALLOWLIST ar could not update the POS        Internal problem
    _FAILED    ni allowlist.
                                                Contact your dealer.
               ng
W01 UPDATE_POS w                                Possible causes:
9   _VATNO     ar                                 vatNo on POS not the same as FDM
               ni                                 values
               ng
                                                Update vatNo on POS.
W02 UPDATE_POS w                                Possible causes:
0   _ESTNO     ar                                 estNo on POS not the same as FDM
               ni                                 values
               ng
                                                Update estNo on POS.


                                       84
E001 BUFFER_FUL er FDM buffer is full.                   User should troubleshoot internet
     L          ro                                       connection, client certificate and trust
                r                                        store. Furthermore, via the e-service of
                                                         FPS Finance, you can check if your
                                                         FDM is properly registered and
                                                         activated.
E002 UNKOWN_P          er POS is not registered for the User should connect a registered POS
     OS                ro FDM.                          to the FDM. You can change the
                       r                                relation between a POS and FDM via
                                                        the e-service of FPS Finance.
E003 FDM_LOCKE er FDM is locked.                         Contact FPS Finance.
     D         ro
               r
E004 TOO_MANY_ er Too many corrupt                       Contact FDM Dealer/Manufacturer
     MEMORY_ER ro transactions because of                (FDM memory problem)
     RORS      r defect memory.


14. Canonical JSON for repeatable hashes
Before creating the hash of the JSON, the following JSON re-encoding should be applied for the
hash to be repeatable.
  Numbers are only allowed to contain the characters 0 through 9, the minus sign (-), and a
  decimal point. This means that values in scientific notation need to be re-encoded in their
  most basic form. Trailing zeroes after the decimal point are not allowed.
  All whitespace as defined in the JSON specifications are removed, no whitespace is added.
  Characters with a code point > 0x1F (31) and < 0x7F (127) are never escaped with the
  exception of code points 0x22 (34 - quotation mark) and 0x5C (92 - reverse solidus) which
  are always escaped as per JSON specification. Those two have to be escaped in their
  shortest possible notation (\" and \\ respectively).
  Characters with a code point <0x20 (32) or > 0x7E (126) are always escaped, and are
  escaped in their shortest possible notation. Backspace, form feed, linefeed, carriage return,
  and horizontal tab have such short notation (\b, \f, \n, \r, and \t respectively).
  When escaping a code point as one or more sets of four hex digits, the hex digits are always
  expressed in uppercase (i.e. \uAAFF).

                                               85
The JSON encoder needs to adhere to a strict implementation of the specifications. For
example: the literals true, false, null need to be in lower case and without quotation marks, the
names of the name/value pairs in the objects need to be wrapped in quotation marks, etc.
The name/value pairs of each object in the JSON document are sorted, in ascending order, on
the numerical value of the code points in the name. Assumed is that no duplicate names are
used since this would be a bad practice as no standard exists in how various JSON decoders
would process them.




                                              86

```
