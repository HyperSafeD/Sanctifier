"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { ThemeToggle } from "./ThemeToggle";

export function NavBar() {
  const pathname = usePathname();
  const [isMenuOpen, setIsMenuOpen] = useState(false);

  const navLinks = [
    { name: "Scan", href: "/scan" },
    { name: "Dashboard", href: "/dashboard" },
    { name: "Playground", href: "/playground" },
    { name: "Terminal", href: "/terminal" },
  ];

  const footerLinks = [
    { name: "Terms of Service", href: "/terms" },
    { name: "Privacy Policy", href: "/privacy" },
  ];

  const isActive = (path: string) => pathname === path;

  return (
    <nav className="sticky top-0 z-50 w-full border-b border-zinc-200 dark:border-zinc-800 bg-white/80 dark:bg-zinc-900/80 backdrop-blur-md theme-high-contrast:bg-black theme-high-contrast:border-white">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between h-16 items-center">
          <div className="flex items-center">
            <Link
              href="/"
              className="text-xl font-bold text-zinc-900 dark:text-zinc-50 theme-high-contrast:text-yellow-300 transition-colors"
            >
              Sanctifier
            </Link>
            <div className="hidden md:ml-10 md:flex md:space-x-8">
              {navLinks.map((link) => (
                <Link
                  key={link.name}
                  href={link.href}
                  className={`inline-flex items-center px-1 pt-1 text-sm font-medium border-b-2 transition-colors ${
                    isActive(link.href)
                      ? "border-zinc-900 dark:border-zinc-100 theme-high-contrast:border-yellow-300 text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                      : "border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 theme-high-contrast:text-white"
                  }`}
                >
                  {link.name}
                </Link>
              ))}
            </div>
          </div>
          <div className="hidden md:flex items-center">
            <ThemeToggle />
          </div>

          {/* Mobile menu button */}
          <div className="flex items-center md:hidden gap-2">
            <ThemeToggle />
            <button
              onClick={() => setIsMenuOpen(!isMenuOpen)}
              className="inline-flex items-center justify-center p-2 rounded-md text-zinc-400 hover:text-zinc-500 hover:bg-zinc-100 dark:hover:bg-zinc-800 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-zinc-500"
              aria-expanded={isMenuOpen}
              aria-controls="mobile-menu"
            >
              <span className="sr-only">Open main menu</span>
              {isMenuOpen ? (
                <svg
                  className="block h-6 w-6"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              ) : (
                <svg
                  className="block h-6 w-6"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    d="M4 6h16M4 12h16M4 18h16"
                  />
                </svg>
              )}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile menu */}
      {isMenuOpen && (
        <div id="mobile-menu" className="md:hidden bg-white dark:bg-zinc-900 border-b border-zinc-200 dark:border-zinc-800 theme-high-contrast:bg-black theme-high-contrast:border-white">
          <div className="pt-2 pb-3 space-y-1 px-4">
            {navLinks.map((link) => (
              <Link
                key={link.name}
                href={link.href}
                onClick={() => setIsMenuOpen(false)}
                className={`block pl-3 pr-4 py-2 border-l-4 text-base font-medium transition-colors ${
                  isActive(link.href)
                    ? "bg-zinc-50 dark:bg-zinc-800 border-zinc-900 dark:border-zinc-100 theme-high-contrast:bg-zinc-900 theme-high-contrast:border-yellow-300 text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                    : "border-transparent text-zinc-500 hover:bg-zinc-50 dark:hover:bg-zinc-800 hover:border-zinc-300 theme-high-contrast:text-white"
                }`}
              >
                {link.name}
              </Link>
            ))}
            <div className="border-t border-zinc-200 dark:border-zinc-700 pt-3 mt-3">
              {footerLinks.map((link) => (
                <Link
                  key={link.name}
                  href={link.href}
                  onClick={() => setIsMenuOpen(false)}
                  className="block pl-3 pr-4 py-2 text-sm text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 theme-high-contrast:text-white"
                >
                  {link.name}
                </Link>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Desktop footer */}
      <div className="hidden md:block border-t border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 theme-high-contrast:bg-black theme-high-contrast:border-white">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex justify-between items-center">
            <p className="text-sm text-zinc-500 dark:text-zinc-400">
              © {new Date().getFullYear()} Sanctifier. All rights reserved.
            </p>
            <div className="flex space-x-6">
              {footerLinks.map((link) => (
                <Link
                  key={link.name}
                  href={link.href}
                  className="text-sm text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-300 theme-high-contrast:text-white"
                >
                  {link.name}
                </Link>
              ))}
            </div>
          </div>
        </div>
      </div>
    </nav>
  );
}
