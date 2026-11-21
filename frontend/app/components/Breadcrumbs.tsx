"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { ChevronRight } from "lucide-react";
import { buildBreadcrumbs, type Crumb } from "../lib/breadcrumbs";

interface BreadcrumbsProps {
  /** Override the auto-generated trail (defaults to one derived from the URL path). */
  items?: Crumb[];
  className?: string;
}

export function Breadcrumbs({ items, className = "" }: BreadcrumbsProps) {
  const pathname = usePathname();
  const crumbs = items ?? buildBreadcrumbs(pathname ?? "/");

  // Nothing to navigate on the home page.
  if (crumbs.length < 2) return null;

  return (
    <nav aria-label="Breadcrumb" className={`max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-4 ${className}`}>
      <ol className="flex flex-wrap items-center gap-1.5 text-sm text-zinc-500 dark:text-zinc-400 theme-high-contrast:text-white">
        {crumbs.map((crumb, index) => {
          const isLast = index === crumbs.length - 1;
          return (
            <li key={crumb.href} className="flex items-center gap-1.5">
              {index > 0 && <ChevronRight className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />}
              {isLast ? (
                <span
                  aria-current="page"
                  className="font-medium text-zinc-900 dark:text-zinc-100 theme-high-contrast:text-yellow-300"
                >
                  {crumb.label}
                </span>
              ) : (
                <Link
                  href={crumb.href}
                  className="hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                >
                  {crumb.label}
                </Link>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
