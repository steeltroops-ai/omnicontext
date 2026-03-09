import Link from "next/link";
import { TOC } from "@/components/toc";

export default function QuickstartPage() {
  const headings = [
    { id: "1-install", text: "1. Install", level: 2 },
    { id: "2-index-your-codebase", text: "2. Index your codebase", level: 2 },
    { id: "3-configure-mcp", text: "3. Configure MCP", level: 2 },
    { id: "4-test", text: "4. Test", level: 2 },
  ];

  return (
    <div className="flex-1 flex h-full">
      <div className="flex-1 px-10 md:px-20 py-16 flex justify-center bg-[#09090B] xl:mr-[240px]">
        <article className="max-w-[760px] w-full">
          <div className="text-[12px] font-semibold tracking-wider uppercase text-zinc-600 mb-6">
            Getting Started
          </div>

          <h1 className="text-4xl md:text-5xl font-semibold text-white tracking-tighter mb-6 leading-tight">
            Quickstart
          </h1>

          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            Get OmniContext running in under 5 minutes.
          </p>

          <h2 id="1-install" className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
            1. Install
          </h2>
          <div className="bg-[#0E0E11] border border-white/5 rounded-xl p-5 font-mono text-[13px] mb-8">
            <div className="text-[10px] text-zinc-600 uppercase tracking-widest mb-3 font-sans font-semibold">
              BASH
            </div>
            <div className="text-zinc-300">
              <div># macOS</div>
              <div>brew install omnicontext</div>
              <div className="mt-3"># Windows</div>
              <div>scoop install omnicontext</div>
              <div className="mt-3"># Linux</div>
              <div>curl -fsSL https://omnicontext.dev/install.sh | bash</div>
            </div>
          </div>

          <h2 id="2-index-your-codebase" className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
            2. Index your codebase
          </h2>
          <div className="bg-[#0E0E11] border border-white/5 rounded-xl p-5 font-mono text-[13px] mb-8">
            <div className="text-[10px] text-zinc-600 uppercase tracking-widest mb-3 font-sans font-semibold">
              BASH
            </div>
            <div className="text-zinc-300">
              <div>cd your-project</div>
              <div>omnicontext index .</div>
            </div>
          </div>

          <h2 id="3-configure-mcp" className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
            3. Configure MCP
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-4 tracking-tight">
            Add OmniContext to your AI assistant's configuration:
          </p>
          <div className="bg-[#0E0E11] border border-white/5 rounded-xl p-5 font-mono text-[13px] mb-8">
            <div className="text-[10px] text-zinc-600 uppercase tracking-widest mb-3 font-sans font-semibold">
              JSON
            </div>
            <div className="text-zinc-300">
              <div>{'{'}</div>
              <div>  "mcpServers": {'{'}</div>
              <div>    "omnicontext": {'{'}</div>
              <div>      "command": "omnicontext-mcp"</div>
              <div>    {'}'}</div>
              <div>  {'}'}</div>
              <div>{'}'}</div>
            </div>
          </div>

          <h2 id="4-test" className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
            4. Test
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-4 tracking-tight">
            Restart your AI assistant and ask:
          </p>
          <div className="bg-[#0E0E11] border border-white/5 rounded-xl p-5 mb-8">
            <p className="text-[15px] text-zinc-300 italic">
              "Search for authentication logic in my codebase"
            </p>
          </div>

          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            Your AI assistant now has semantic code search.
          </p>

          <div className="flex justify-between mt-12 pb-16">
            <Link
              href="/docs"
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-full border border-white/10 text-zinc-300 text-[14px] font-semibold hover:bg-white/5 transition-all duration-300"
            >
              ← Introduction
            </Link>
            <Link
              href="/docs/installation"
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-zinc-100 text-black text-[14px] font-semibold hover:scale-105 active:scale-95 transition-all duration-300"
            >
              Installation →
            </Link>
          </div>
        </article>
      </div>

      {/* Right TOC Sidebar */}
      <TOC headings={headings} />
    </div>
  );
}
