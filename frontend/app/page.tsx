import Link from "next/link";

export default function Home() {
  return (
    <div className="relative min-h-[calc(100vh-64px)] flex flex-col items-center overflow-hidden bg-white dark:bg-zinc-950 font-sans">
      {/* Decorative background Elements */}
      <div className="absolute inset-0 z-0 overflow-hidden pointer-events-none">
        <div className="absolute top-[10%] left-[-10%] w-[40%] h-[40%] rounded-full bg-emerald-500/10 blur-[120px]" />
        <div className="absolute bottom-[10%] right-[-10%] w-[40%] h-[40%] rounded-full bg-blue-500/10 blur-[120px]" />
        <div className="absolute inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:40px_40px] [mask-image:radial-gradient(ellipse_60%_50%_at_50%_0%,#000_70%,transparent_100%)]" />
      </div>

      <main className="relative z-10 flex flex-col items-center w-full max-w-7xl px-6 pt-20 pb-32">
        {/* Welcome Badge */}
        <div className="inline-flex items-center rounded-full border border-emerald-500/20 bg-emerald-500/5 px-3 py-1 mb-6 text-sm font-medium text-emerald-600 dark:text-emerald-400 backdrop-blur-sm animate-in fade-in slide-in-from-bottom-3 duration-1000">
          <span className="relative flex h-2 w-2 mr-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
            <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
          </span>
          v0.1.0 Alpha is now live
        </div>

        {/* Hero Section */}
        <div className="text-center space-y-8 max-w-4xl mx-auto">
          <h1 className="text-5xl md:text-7xl font-extrabold tracking-tight text-zinc-900 dark:text-zinc-50 leading-[1.1]">
            Sanctify Your <br />
            <span className="bg-gradient-to-r from-emerald-500 via-blue-500 to-indigo-600 bg-clip-text text-transparent">
              Soroban Smart Contracts
            </span>
          </h1>
          <p className="text-lg md:text-xl text-zinc-600 dark:text-zinc-400 text-center max-w-2xl mx-auto leading-relaxed">
            Advanced static analysis, formal verification, and security auditing tools
            purpose-built for the Stellar network. High-performance, real-time security
            for the next generation of DeFi.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4 pt-4">
            <Link
              href="/scan"
              className="group relative w-full sm:w-auto overflow-hidden rounded-2xl bg-zinc-900 dark:bg-zinc-100 px-8 py-4 font-bold text-white dark:text-zinc-900 transition-all hover:scale-105 active:scale-95 shadow-xl shadow-emerald-500/10"
            >
              <span className="relative z-10 flex items-center justify-center gap-2">
                Start Security Scan
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="transition-transform group-hover:translate-x-1"><path d="M5 12h14" /><path d="m12 5 7 7-7 7" /></svg>
              </span>
            </Link>
            <Link
              href="/dashboard"
              className="w-full sm:w-auto rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-white/50 dark:bg-zinc-900/50 px-8 py-4 font-bold backdrop-blur-md transition-all hover:bg-zinc-100 dark:hover:bg-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700 active:scale-95"
            >
              View Reports
            </Link>
          </div>
        </div>

        {/* Feature Grid */}
        <div className="mt-32 grid grid-cols-1 md:grid-cols-3 gap-8 w-full">
          <div className="group p-8 rounded-3xl border border-zinc-200 dark:border-zinc-800 bg-white/50 dark:bg-zinc-900/50 backdrop-blur-sm transition-all hover:border-emerald-500/30 hover:shadow-2xl hover:shadow-emerald-500/5">
            <div className="w-12 h-12 rounded-2xl bg-emerald-500/10 flex items-center justify-center text-emerald-500 mb-6 group-hover:scale-110 transition-transform">
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg>
            </div>
            <h3 className="text-xl font-bold mb-3 text-zinc-900 dark:text-zinc-50">Deep Static Analysis</h3>
            <p className="text-zinc-600 dark:text-zinc-400 leading-relaxed">
              Detect common vulnerabilities like reentrancy, integer overflows, and authorization gaps automatically.
            </p>
          </div>

          <div className="group p-8 rounded-3xl border border-zinc-200 dark:border-zinc-800 bg-white/50 dark:bg-zinc-900/50 backdrop-blur-sm transition-all hover:border-blue-500/30 hover:shadow-2xl hover:shadow-blue-500/5">
            <div className="w-12 h-12 rounded-2xl bg-blue-500/10 flex items-center justify-center text-blue-500 mb-6 group-hover:scale-110 transition-transform">
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></svg>
            </div>
            <h3 className="text-xl font-bold mb-3 text-zinc-900 dark:text-zinc-50">Real-time Verification</h3>
            <p className="text-zinc-600 dark:text-zinc-400 leading-relaxed">
              Watch analysis logs stream in real-time as the engine explores contract execution paths.
            </p>
          </div>

          <div className="group p-8 rounded-3xl border border-zinc-200 dark:border-zinc-800 bg-white/50 dark:bg-zinc-900/50 backdrop-blur-sm transition-all hover:border-indigo-500/30 hover:shadow-2xl hover:shadow-indigo-500/5">
            <div className="w-12 h-12 rounded-2xl bg-indigo-500/10 flex items-center justify-center text-indigo-500 mb-6 group-hover:scale-110 transition-transform">
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 2v20" /><path d="m17 7-5-5-5 5" /><path d="M17 17l-5 5-5-5" /></svg>
            </div>
            <h3 className="text-xl font-bold mb-3 text-zinc-900 dark:text-zinc-50">Protocol Compliance</h3>
            <p className="text-zinc-600 dark:text-zinc-400 leading-relaxed">
              Ensure your contracts adhere to CAPs and Stellar ecosystem best practices effortlessly.
            </p>
          </div>
        </div>

        {/* CTA Footer */}
        <div className="mt-40 text-center space-y-6">
          <h2 className="text-3xl font-bold text-zinc-900 dark:text-zinc-50">Ready to secure your protocols?</h2>
          <p className="text-zinc-500 dark:text-zinc-400">Join forward-thinking developers building the future of Stellar.</p>
          <Link
            href="/scan"
            className="inline-flex h-12 items-center justify-center rounded-xl bg-zinc-900 dark:bg-zinc-100 px-8 text-sm font-bold text-white dark:text-zinc-900 transition-colors hover:bg-zinc-800 dark:hover:bg-zinc-200"
          >
            Launch Scanner
          </Link>
        </div>
      </main>

    </div>
  );
}
