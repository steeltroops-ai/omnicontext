export default function PrivacyPage() {
    return (
        <div className="min-h-screen bg-[#09090B] text-white">
            <div className="max-w-4xl mx-auto px-6 py-24">
                <h1 className="text-4xl font-semibold mb-8">Privacy Policy</h1>

                <div className="prose prose-invert max-w-none space-y-6 text-zinc-400">
                    <p className="text-lg">
                        Last updated: {new Date().toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' })}
                    </p>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Overview</h2>
                        <p>
                            OmniContext is committed to protecting your privacy. This Privacy Policy explains how we collect, use, and safeguard your information when you use our software and services.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Data Collection</h2>
                        <p>
                            OmniContext operates entirely on your local machine. We do not collect, transmit, or store any of your code, data, or personal information on external servers.
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>All code indexing happens locally on your machine</li>
                            <li>No code or embeddings are sent to external servers</li>
                            <li>No telemetry or usage data is collected by default</li>
                            <li>Your codebase never leaves your machine</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Local Processing</h2>
                        <p>
                            OmniContext uses local ONNX models for embedding generation. All processing occurs on your machine:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Embeddings are generated locally using CPU/GPU</li>
                            <li>Vector indices are stored in local SQLite databases</li>
                            <li>No external API calls are made for code analysis</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Third-Party Services</h2>
                        <p>
                            OmniContext may integrate with third-party AI services (Claude Desktop, Cursor, etc.) through the Model Context Protocol (MCP). When using these integrations:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>You control what context is shared with AI services</li>
                            <li>Context is only sent when you explicitly request it</li>
                            <li>Third-party services have their own privacy policies</li>
                            <li>We recommend reviewing their policies before use</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Website Analytics</h2>
                        <p>
                            Our website (omnicontext.dev) may use standard web analytics to understand usage patterns. This data is anonymized and does not include any code or personal information from the OmniContext software itself.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Open Source</h2>
                        <p>
                            OmniContext is open source software. You can review our source code to verify our privacy claims:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Source code: <a href="https://github.com/steeltroops-ai/omnicontext" className="text-emerald-400 hover:text-emerald-300">github.com/steeltroops-ai/omnicontext</a></li>
                            <li>All data processing is transparent and auditable</li>
                            <li>No hidden telemetry or tracking</li>
                        </ul>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Changes to This Policy</h2>
                        <p>
                            We may update this Privacy Policy from time to time. We will notify users of any material changes by updating the "Last updated" date at the top of this policy.
                        </p>
                    </section>

                    <section className="mt-8">
                        <h2 className="text-2xl font-semibold text-white mb-4">Contact Us</h2>
                        <p>
                            If you have questions about this Privacy Policy, please contact us:
                        </p>
                        <ul className="list-disc list-inside space-y-2 mt-4">
                            <li>Email: privacy@omnicontext.dev</li>
                            <li>GitHub: <a href="https://github.com/steeltroops-ai/omnicontext/issues" className="text-emerald-400 hover:text-emerald-300">github.com/steeltroops-ai/omnicontext/issues</a></li>
                        </ul>
                    </section>
                </div>
            </div>
        </div>
    );
}
