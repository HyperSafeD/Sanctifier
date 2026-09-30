import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Terms of Service | Sanctifier",
  description: "Terms of Service for the Sanctifier security analysis platform",
};

export default function TermsPage() {
  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 py-16">
        <div className="prose prose-zinc dark:prose-invert max-w-none">
          <h1 className="text-4xl font-bold mb-8">Terms of Service</h1>
          
          <p className="text-sm text-zinc-500 mb-8">Last updated: {new Date().toLocaleDateString()}</p>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">1. Acceptance of Terms</h2>
            <p>
              By accessing or using the Sanctifier security analysis platform ("Service"), you agree to be bound by these Terms of Service ("Terms"). 
              If you do not agree to these Terms, you may not use the Service.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">2. Description of Service</h2>
            <p>
              Sanctifier provides automated security analysis tools for Soroban smart contracts. The Service analyzes uploaded contract source code 
              and generates security reports identifying potential vulnerabilities, code quality issues, and compliance concerns.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">3. User Responsibilities</h2>
            <p>As a user of the Service, you agree to:</p>
            <ul>
              <li>Only upload contract source code that you have the right to analyze</li>
              <li>Not use the Service for any illegal or unauthorized purpose</li>
              <li>Not attempt to circumvent any security measures or rate limits</li>
              <li>Not upload malicious code designed to harm the Service or its infrastructure</li>
              <li>Comply with all applicable laws and regulations</li>
            </ul>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">4. Data and Content</h2>
            <p>
              You retain ownership of all contract source code and data you upload to the Service. By uploading content, you grant us a license 
              to process, analyze, and store the content solely for the purpose of providing the security analysis service.
            </p>
            <p className="mt-4">
              For details on how we handle your data, including retention policies, please refer to our Privacy Policy.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">5. Security Analysis Results</h2>
            <p>
              Security analysis results provided by the Service are for informational purposes only. The Service does not guarantee:
            </p>
            <ul>
              <li>That all vulnerabilities will be detected</li>
              <li>That reported issues are actual security vulnerabilities</li>
              <li>That your contract is free from all security risks</li>
              <li>That the analysis is suitable for production deployment decisions</li>
            </ul>
            <p className="mt-4">
              You are solely responsible for validating analysis results and conducting additional security reviews before deploying contracts to mainnet.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">6. Limitation of Liability</h2>
            <p>
              TO THE MAXIMUM EXTENT PERMITTED BY APPLICABLE LAW, SANCTIFIER SHALL NOT BE LIABLE FOR ANY INDIRECT, INCIDENTAL, SPECIAL, 
              CONSEQUENTIAL, OR PUNITIVE DAMAGES, INCLUDING WITHOUT LIMITATION, LOSS OF PROFITS, DATA, USE, GOODWILL, OR OTHER INTANGIBLE 
              LOSSES, RESULTING FROM YOUR USE OF THE SERVICE.
            </p>
            <p className="mt-4">
              IN NO EVENT SHALL SANCTIFIER'S TOTAL LIABILITY TO YOU FOR ALL CLAIMS RELATED TO THE SERVICE EXCEED THE AMOUNT YOU PAID, 
              IF ANY, FOR USING THE SERVICE DURING THE PRECEDING TWELVE (12) MONTHS.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">7. Disclaimer of Warranties</h2>
            <p>
              THE SERVICE IS PROVIDED "AS IS" AND "AS AVAILABLE" WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING, 
              BUT NOT LIMITED TO, IMPLIED WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE, AND NON-INFRINGEMENT.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">8. Service Availability</h2>
            <p>
              We do not guarantee uninterrupted or error-free operation of the Service. We reserve the right to modify, suspend, or discontinue 
              the Service at any time without notice.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">9. Acceptable Use Policy</h2>
            <p>You may not use the Service to:</p>
            <ul>
              <li>Analyze contracts containing malicious code or malware</li>
              <li>Attempt to reverse engineer or compromise the Service</li>
              <li>Upload excessively large files or abuse rate limits</li>
              <li>Use the Service to compete with Sanctifier</li>
              <li>Violate any third-party rights or applicable laws</li>
            </ul>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">10. Indemnification</h2>
            <p>
              You agree to indemnify and hold harmless Sanctifier and its affiliates from any claims arising from your use of the Service, 
              your violation of these Terms, or your violation of any third-party rights.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">11. Modifications to Terms</h2>
            <p>
              We reserve the right to modify these Terms at any time. We will notify users of material changes by posting the updated Terms 
              on the Service. Your continued use of the Service after such modifications constitutes your acceptance of the updated Terms.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">12. Termination</h2>
            <p>
              We may terminate or suspend your access to the Service at any time, with or without cause, with or without notice. 
              Upon termination, your right to use the Service will immediately cease.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">13. Governing Law</h2>
            <p>
              These Terms shall be governed by and construed in accordance with the laws of the jurisdiction in which Sanctifier is established, 
              without regard to its conflict of law provisions.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">14. Contact Information</h2>
            <p>
              For questions about these Terms, please contact us at:
            </p>
            <p className="mt-2">
              Email: legal@sanctifier.dev<br />
              GitHub: https://github.com/alfeedrips/Sanctifier/issues
            </p>
          </section>

          <div className="mt-12 p-6 bg-zinc-100 dark:bg-zinc-800 rounded-lg">
            <p className="text-sm text-zinc-600 dark:text-zinc-400">
              <strong>Legal Notice:</strong> These Terms of Service are provided for informational purposes and do not constitute formal legal advice. 
              For mainnet deployments involving significant value, we recommend consulting with legal counsel to ensure compliance with applicable regulations.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
