# Data Processing Agreement (DPA)

**Version:** 1.0  
**Effective Date:** July 2026  
**Between:** Sanctifier ("Processor") and the Customer identified in the applicable Order Form ("Controller").

---

## 1. Definitions

- **"Data Protection Law"** means Regulation (EU) 2016/679 (GDPR), the California Consumer Privacy Act (CCPA), and any other applicable privacy or data protection legislation.
- **"Personal Data"** means any information relating to an identified or identifiable natural person processed under this Agreement.
- **"Processing"** means any operation performed on Personal Data, including collection, storage, analysis, and deletion.
- **"Sub-processor"** means any third party engaged by Processor to assist in Processing Personal Data.

## 2. Scope and Purpose

This DPA governs the Processing of Personal Data by Sanctifier when Controller uses the Sanctifier hosted dashboard and CLI for smart-contract security scanning. The Processing is limited to:

- Account identifiers (email address, wallet/public-key addresses).
- Source-code hashes and analysis metadata (rule IDs, timestamps, file hashes).
- Scan result summaries transmitted to webhook endpoints configured by Controller.

## 3. Processor Obligations

3.1 **Processing Instructions.** Processor shall Process Personal Data only on documented instructions from Controller, unless required otherwise by Union or Member State law.

3.2 **Confidentiality.** Processor shall ensure that personnel authorized to Process Personal Data are bound by appropriate confidentiality obligations.

3.3 **Data Minimisation.** Processor collects only the data necessary to perform contract-security analysis as described in the Privacy Policy.

3.4 **Retention.** Personal Data is retained for the duration of the Controller's active subscription and deleted within 90 days of termination, except where longer retention is required by law.

## 4. Sub-processors

4.1 **Current Sub-processors.** Processor uses the following sub-processors:

| Sub-processor | Service | Location |
|---|---|---|
| GitHub, Inc. | Source-code hosting and release distribution | United States |
| Stellar Development Foundation | Soroban ledger queries | Global (decentralised) |

4.2 **Engagement.** Processor shall notify Controller at least 30 days before engaging any new sub-processor. Controller may object on reasonable grounds related to data protection.

## 5. Data Breach Notification

5.1 Processor shall notify Controller without undue delay (and within 72 hours of becoming aware) of any breach of security leading to accidental or unlawful destruction, loss, alteration, or unauthorised disclosure of Personal Data.

5.2 Notification shall include: (a) the nature of the breach; (b) the categories and approximate number of data subjects and records affected; (c) contact details for the data protection officer or other responsible contact; and (d) measures taken or proposed to address the breach.

## 6. Data Subject Rights

Processor shall assist Controller by appropriate technical and organisational measures to fulfil Controller's obligation to respond to data subject access, rectification, erasure, portability, and objection requests.

## 7. Audit

7.1 Upon request and at reasonable intervals, Processor shall make available all information necessary to demonstrate compliance with this DPA.

7.2 Processor shall inform Controller if, in its opinion, an instruction infringes Data Protection Law.

## 8. Deletion and Return

Within 90 days of termination of the service, Processor shall delete all Personal Data processed on behalf of Controller, unless retention is required by applicable law.

## 9. Governing Law

This DPA shall be governed by the laws of England and Wales, without regard to conflict-of-laws principles.

---

**To request an executed copy of this DPA, contact sanctifier@example.com.**
