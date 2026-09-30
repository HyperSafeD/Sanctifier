import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Privacy Policy | Sanctifier",
  description: "Privacy Policy for the Sanctifier security analysis platform",
};

export default function PrivacyPage() {
  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 py-16">
        <div className="prose prose-zinc dark:prose-invert max-w-none">
          <h1 className="text-4xl font-bold mb-8">Privacy Policy</h1>
          
          <p className="text-sm text-zinc-500 mb-8">Last updated: {new Date().toLocaleDateString()}</p>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">1. Introduction</h2>
            <p>
              Sanctifier ("we", "our", or "us") is committed to protecting your privacy. This Privacy Policy explains how we collect, 
              use, store, and protect your information when you use our security analysis platform for Soroban smart contracts.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">2. Information We Collect</h2>
            
            <h3 className="text-xl font-semibold mt-6 mb-3">2.1 Contract Source Code</h3>
            <p>
              When you upload contract source files for analysis, we collect:
            </p>
            <ul>
              <li>Rust source code files (.rs)</li>
              <li>Contract configuration files (e.g., Cargo.toml)</li>
              <li>Custom rules files (if provided)</li>
            </ul>
            <p className="mt-4">
              <strong>Purpose:</strong> To perform security analysis and generate vulnerability reports.
            </p>

            <h3 className="text-xl font-semibold mt-6 mb-3">2.2 Analysis Results</h3>
            <p>
              We generate and may store:
            </p>
            <ul>
              <li>Security findings and vulnerability reports</li>
              <li>Analysis metrics (duration, rules matched)</li>
              <li>Sanctity scores and risk assessments</li>
            </ul>
            <p className="mt-4">
              <strong>Purpose:</strong> To provide you with analysis results and enable report sharing.
            </p>

            <h3 className="text-xl font-semibold mt-6 mb-3">2.3 Telemetry Data (Optional)</h3>
            <p>
              If you explicitly enable telemetry, we collect:
            </p>
            <ul>
              <li>Rule IDs that matched during scans</li>
              <li>Total analysis duration in milliseconds</li>
              <li>Sanitized tool version information</li>
            </ul>
            <p className="mt-4">
              <strong>We do NOT collect:</strong> Source code, file paths, contract names, repository URLs, or any credentials.
            </p>
            <p className="mt-4">
              <strong>Purpose:</strong> To improve our analysis engine and track feature usage.
            </p>
            <p className="mt-4">
              <strong>Opt-in:</strong> Telemetry is disabled by default. You must explicitly enable it in settings.
            </p>

            <h3 className="text-xl font-semibold mt-6 mb-3">2.4 Account Information</h3>
            <p>
              If you create an account, we may collect:
            </p>
            <ul>
              <li>Email address (for authentication and notifications)</li>
              <li>Username/display name</li>
              <li>Authentication tokens</li>
            </ul>
            <p className="mt-4">
              <strong>Purpose:</strong> To provide account management and personalized services.
            </p>

            <h3 className="text-xl font-semibold mt-6 mb-3">2.5 Technical Data</h3>
            <p>
              We automatically collect:
            </p>
            <ul>
              <li>IP address (for rate limiting and security)</li>
              <li>Browser type and version</li>
              <li>Operating system</li>
              <li>Referring website</li>
            </ul>
            <p className="mt-4">
              <strong>Purpose:</strong> To ensure service availability, prevent abuse, and improve user experience.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">3. Data Retention</h2>
            
            <h3 className="text-xl font-semibold mt-6 mb-3">3.1 Contract Source Code</h3>
            <p>
              Uploaded contract source code is retained for the following periods:
            </p>
            <ul>
              <li><strong>Temporary analysis:</strong> Deleted immediately after analysis completion (ephemeral processing)</li>
              <li><strong>With saved reports:</strong> Retained for 30 days from the analysis date</li>
              <li><strong>Shared reports:</strong> Retained for the duration of the share link (default 7 days, maximum 30 days)</li>
            </ul>
            <p className="mt-4">
              You can request immediate deletion of your contract code at any time through the dashboard or by contacting support.
            </p>

            <h3 className="text-xl font-semibold mt-6 mb-3">3.2 Analysis Results</h3>
            <p>
              Analysis reports and findings are retained according to your account tier:
            </p>
            <ul>
              <li><strong>Free tier:</strong> Reports retained for 7 days</li>
              <li><strong>Paid tier:</strong> Reports retained for 90 days</li>
              <li><strong>Enterprise:</strong> Custom retention periods available</li>
            </ul>

            <h3 className="text-xl font-semibold mt-6 mb-3">3.3 Telemetry Data</h3>
            <p>
              Telemetry data is retained for a maximum of 90 days for analysis purposes, after which it is aggregated and anonymized.
            </p>

            <h3 className="text-xl font-semibold mt-6 mb-3">3.4 Account Data</h3>
            <p>
              Account information is retained until you delete your account. Upon account deletion, all personal data is permanently removed within 30 days.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">4. How We Use Your Information</h2>
            <p>We use your information to:</p>
            <ul>
              <li>Perform security analysis on uploaded contracts</li>
              <li>Generate and display analysis reports</li>
              <li>Improve our analysis algorithms (using anonymized telemetry)</li>
              <li>Provide customer support</li>
              <li>Prevent abuse and ensure service security</li>
              <li>Comply with legal obligations</li>
            </ul>
            <p className="mt-4">
              We do not sell, rent, or license your contract source code to third parties.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">5. Data Security</h2>
            <p>
              We implement industry-standard security measures to protect your information:
            </p>
            <ul>
              <li><strong>Encryption:</strong> Data is encrypted in transit (TLS 1.3) and at rest (AES-256)</li>
              <li><strong>Access controls:</strong> Strict access controls and authentication for all systems</li>
              <li><strong>Isolation:</strong> Customer data is logically isolated in multi-tenant environments</li>
              <li><strong>Auditing:</strong> Comprehensive logging and monitoring of data access</li>
              <li><strong>Testing:</strong> Regular security audits and penetration testing</li>
            </ul>
            <p className="mt-4">
              However, no security measure is completely foolproof. We cannot guarantee absolute security.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">6. Your Rights</h2>
            <p>
              Depending on your jurisdiction, you may have the following rights:
            </p>
            <ul>
              <li><strong>Access:</strong> Request a copy of your personal data</li>
              <li><strong>Deletion:</strong> Request deletion of your data (with some exceptions)</li>
              <li><strong>Correction:</strong> Request correction of inaccurate data</li>
              <li><strong>Portability:</strong> Request transfer of your data to another service</li>
              <li><strong>Objection:</strong> Object to processing of your data</li>
              <li><strong>Restriction:</strong> Request restriction of data processing</li>
            </ul>
            <p className="mt-4">
              To exercise these rights, contact us at privacy@sanctifier.dev
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">7. Data Sharing</h2>
            <p>
              We do not share your contract source code with third parties, except in the following circumstances:
            </p>
            <ul>
              <li><strong>Service providers:</strong> With trusted third-party service providers who assist in operating our service (e.g., cloud infrastructure, analytics)</li>
              <li><strong>Legal requirements:</strong> When required by law, court order, or government request</li>
              <li><strong>Business transfer:</strong> In connection with a merger, acquisition, or sale of assets</li>
              <li><strong>With consent:</strong> With your explicit consent</li>
            </ul>
            <p className="mt-4">
              All third-party service providers are contractually bound to protect your data and may only use it for specified purposes.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">8. International Data Transfers</h2>
            <p>
              Your information may be transferred to and processed in countries other than your country of residence. 
              We ensure appropriate safeguards are in place to protect your data in accordance with this Privacy Policy 
              and applicable data protection laws.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">9. Children's Privacy</h2>
            <p>
              Our service is not intended for children under the age of 18. We do not knowingly collect personal information 
              from children. If we become aware that we have collected such information, we will take steps to delete it.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">10. Changes to This Policy</h2>
            <p>
              We may update this Privacy Policy from time to time. We will notify users of material changes by:
            </p>
            <ul>
              <li>Posting the updated policy on our website</li>
              <li>Sending email notifications to registered users</li>
              <li>Displaying a prominent notice in the dashboard</li>
            </ul>
            <p className="mt-4">
              Your continued use of the service after such changes constitutes acceptance of the updated policy.
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">11. GDPR Compliance</h2>
            <p>
              For users in the European Union, we comply with the General Data Protection Regulation (GDPR). 
              Our legal basis for processing includes:
            </p>
            <ul>
              <li><strong>Contract performance:</strong> Providing security analysis services</li>
              <li><strong>Legitimate interests:</strong> Improving our service and preventing abuse</li>
              <li><strong>Consent:</strong> When you explicitly opt-in to telemetry or other optional features</li>
              <li><strong>Legal obligation:</strong> When required by law</li>
            </ul>
            <p className="mt-4">
              EU users may contact our EU representative at: [EU Representative Address]
            </p>
          </section>

          <section className="mb-8">
            <h2 className="text-2xl font-semibold mb-4">12. Contact Information</h2>
            <p>
              For questions about this Privacy Policy or our data practices, please contact:
            </p>
            <p className="mt-2">
              Email: privacy@sanctifier.dev<br />
              GitHub: https://github.com/alfeedrips/Sanctifier/issues
            </p>
          </section>

          <div className="mt-12 p-6 bg-zinc-100 dark:bg-zinc-800 rounded-lg">
            <p className="text-sm text-zinc-600 dark:text-zinc-400">
              <strong>Data Processing Agreement:</strong> For enterprise customers requiring a formal Data Processing Agreement (DPA) 
              under GDPR or other regulations, please contact us at legal@sanctifier.dev. See issue #1162 for the DPA template.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
