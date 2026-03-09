export default function TermsPage() {
    return (
        <div className="min-h-screen bg-[#09090B] text-white">
            <div className="max-w-4xl mx-auto px-6 py-24">
                <h1 className="text-4xl font-semibold mb-8">Terms of Service</h1>

                <div className="prose prose-invert max-w-none space-y-6 text-zinc-400">
                    <p className="text-lg">
                        Last updated: {new Date().toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' })}
                    </p>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Acceptance of Terms</h2>
                        <p>
                            By accessing or using OmniContext software and services, you agree to be bound by these Terms of Service. If you do not agree to these terms, please do not use our software.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">License</h2>
                        <p>
                            OmniContext is open source software licensed under the MIT License. You are free to:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Use the software for personal or commercial purposes</li>
                            <li>Modify the source code</li>
                            <li>Distribute copies of the software</li>
                            <li>Sublicense the software</li>
                        </ul>
                        <p className="mt-4">
                            The software is provided "as is", without warranty of any kind, express or implied.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Acceptable Use</h2>
                        <p>
                            You agree to use OmniContext in compliance with all applicable laws and regulations. You may not:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Use the software for any illegal or unauthorized purpose</li>
                            <li>Attempt to reverse engineer or decompile the software (except as permitted by open source license)</li>
                            <li>Use the software to violate the privacy or rights of others</li>
                            <li>Distribute malware or harmful code through the software</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Third-Party Integrations</h2>
                        <p>
                            OmniContext integrates with third-party AI services through the Model Context Protocol (MCP). When using these integrations:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>You are subject to the terms of service of those third-party providers</li>
                            <li>We are not responsible for the actions or policies of third-party services</li>
                            <li>You are responsible for any data you share with third-party services</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Disclaimer of Warranties</h2>
                        <p>
                            OmniContext is provided "as is" and "as available" without warranties of any kind, either express or implied, including but not limited to:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Merchantability</li>
                            <li>Fitness for a particular purpose</li>
                            <li>Non-infringement</li>
                            <li>Accuracy or completeness of results</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Limitation of Liability</h2>
                        <p>
                            To the maximum extent permitted by law, OmniContext and its contributors shall not be liable for any indirect, incidental, special, consequential, or punitive damages, or any loss of profits or revenues, whether incurred directly or indirectly, or any loss of data, use, goodwill, or other intangible losses.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Indemnification</h2>
                        <p>
                            You agree to indemnify and hold harmless OmniContext and its contributors from any claims, damages, losses, liabilities, and expenses (including legal fees) arising from your use of the software or violation of these terms.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Changes to Terms</h2>
                        <p>
                            We reserve the right to modify these Terms of Service at any time. We will notify users of material changes by updating the "Last updated" date. Continued use of the software after changes constitutes acceptance of the new terms.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Governing Law</h2>
                        <p>
                            These Terms of Service shall be governed by and construed in accordance with the laws of the jurisdiction in which OmniContext operates, without regard to its conflict of law provisions.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Contact Information</h2>
                        <p>
                            If you have questions about these Terms of Service, please contact us:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Email: legal@omnicontext.dev</li>
                            <li>GitHub: <a href="https://github.com/steeltroops-ai/omnicontext/issues" className="text-emerald-400 hover:text-emerald-300">github.com/steeltroops-ai/omnicontext/issues</a></li>
                        </ul>
                    </section>
                </div>
            </div>
        </div>
    );
}
