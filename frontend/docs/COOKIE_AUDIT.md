# Cookie & Tracking Audit

**Date**: July 29, 2026  
**Auditor**: @dev-susa  
**Issue**: #1164  
**Version**: Frontend v0.1.1

## Executive Summary

**Finding**: ✅ **No non-essential cookies or tracking scripts detected**

The Sanctifier dashboard currently sets **zero cookies** and uses **zero third-party tracking/analytics services**. A cookie consent banner is **not required** under current GDPR/ePrivacy regulations.

---

## Audit Methodology

### 1. Code Analysis
- ✅ Reviewed `frontend/app/layout.tsx` (root layout)
- ✅ Scanned all `*.ts`, `*.tsx`, `*.js`, `*.jsx` files
- ✅ Checked `package.json` dependencies
- ✅ Inspected `next.config.ts` CSP headers
- ✅ Searched for common tracking SDKs (Sentry, Google Analytics, PostHog, Mixpanel, Amplitude, Segment)

### 2. Runtime Testing
- ✅ Launched `npm run dev`
- ✅ Opened DevTools → Application → Cookies
- ✅ Navigated through all pages (home, upload, results, etc.)
- ✅ Monitored network requests for third-party domains

### 3. Third-Party Script Review
- ✅ No `<script>` tags pointing to external CDNs
- ✅ CSP blocks external scripts (`script-src 'self'`)
- ✅ No analytics libraries in `package.json`

---

## Detailed Findings

### Cookies Set: **0**

| Name | Domain | Purpose | Category | Expiry | EU Consent Required? |
|------|--------|---------|----------|--------|---------------------|
| *None* | - | - | - | - | - |

**Result**: ✅ No cookies detected

---

### localStorage / sessionStorage Usage

| Key | Purpose | Category | Contains PII? | Consent Required? |
|-----|---------|----------|---------------|-------------------|
| `theme` | User's theme preference (light/dark) | **Essential** | No | ❌ No (essential functionality) |

**Details**:
- Set by: `frontend/app/layout.tsx` theme bootstrap script
- Value: `"light"` or `"dark"`
- Purpose: Prevent flash of unstyled content on page load
- GDPR Classification: **Essential** (functional preference)

**Result**: ✅ No consent required for theme preference

---

### Third-Party Scripts & SDKs

| Service | Integrated? | Sets Cookies? | Purpose | Consent Required? |
|---------|-------------|---------------|---------|-------------------|
| **Sentry** | ❌ No | N/A | Error tracking (#1148 closed but not implemented) | Would require consent |
| **Google Analytics** | ❌ No | N/A | - | - |
| **PostHog** | ❌ No | N/A | - | - |
| **Mixpanel** | ❌ No | N/A | - | - |
| **Amplitude** | ❌ No | N/A | - | - |
| **Segment** | ❌ No | N/A | - | - |
| **Vercel Analytics** | ❌ No | N/A | - | - |

**Result**: ✅ Zero third-party tracking services detected

---

### Network Requests Audit

**External Domains Called**:
- ✅ None detected (all requests to `'self'`)

**CSP Policy**:
```
default-src 'self';
style-src 'self' 'unsafe-inline' (dev only);
script-src 'self';
img-src 'self' data: https:;
font-src 'self' data:;
connect-src 'self';
frame-ancestors 'none';
```

The Content Security Policy **blocks** external script loading, which would prevent unauthorized tracking even if accidentally added.

---

## GDPR/ePrivacy Compliance Status

### EU ePrivacy Directive (Cookie Law)

**Requirement**: Consent required before setting **non-essential cookies**

**Current Status**: ✅ **Compliant** - No non-essential cookies set

**Essential Cookies** (no consent needed):
- Authentication/session management ❌ Not used
- Load balancing ❌ Not used  
- User interface preferences (theme) ✅ **Used (localStorage only, not a cookie)**

---

### GDPR (General Data Protection Regulation)

**Requirement**: Lawful basis for processing personal data

**Current Status**: ✅ **Compliant** - No personal data collected or tracked

**Data Processing Activities**:
| Activity | Data Processed | Lawful Basis | Storage | Consent Needed? |
|----------|----------------|--------------|---------|-----------------|
| Theme preference | Light/dark mode choice | Legitimate interest (UX) | localStorage (local only) | ❌ No |
| Contract upload | Contract source code | User-initiated task | Memory only (not persisted) | ❌ No |
| Analysis results | Security findings | User-initiated task | Session only | ❌ No |

**Result**: ✅ No consent banner required under current configuration

---

## Future Considerations

### If Analytics Are Added Later

If #1148 (Sentry) or similar analytics tools are implemented, **consent will be required** for:

#### **Non-Essential Cookies**:
- Sentry session tracking cookies
- Google Analytics `_ga`, `_gid`, `_gat` cookies
- Any performance monitoring cookies

#### **Recommended Implementation**:
1. **Use `<CookieConsentBanner />` component** (see `frontend/app/components/CookieConsentBanner.tsx`)
2. **Gate analytics initialization** behind consent
3. **Update Privacy Policy** (#1158) with cookie table
4. **Provide cookie preference center** for users to revoke consent

**Example Integration**:
```tsx
// frontend/app/layout.tsx
import { CookieConsentBanner } from './components/CookieConsentBanner';

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html>
      <body>
        {children}
        <CookieConsentBanner />
      </body>
    </html>
  );
}
```

**Consent Gating Example**:
```tsx
// Only initialize Sentry if user consented
if (hasConsent('analytics')) {
  Sentry.init({ dsn: process.env.NEXT_PUBLIC_SENTRY_DSN });
}
```

---

## Documentation Updates

### Privacy Policy (#1158)

**Recommended Section**:

> **Cookies and Tracking**
>
> The Sanctifier dashboard does not currently use cookies or third-party tracking scripts. We store your theme preference (light/dark mode) locally in your browser's localStorage for a better user experience. This data never leaves your device.
>
> If we introduce analytics or error tracking in the future, we will update this policy and implement a cookie consent banner before collecting any data.

---

## Verification Steps

To reproduce this audit:

### 1. Check for Cookies
```bash
# Start the dev server
cd frontend
npm run dev

# Open browser DevTools → Application → Cookies
# Navigate to http://localhost:3000
# Verify: No cookies listed
```

### 2. Check localStorage
```js
// In browser console:
localStorage.getItem('theme') // Should return "light" or "dark"
localStorage.length // Should return 1
```

### 3. Check Network Requests
```bash
# In DevTools → Network tab
# Filter by "XHR" and "Fetch"
# Verify: All requests go to localhost:3000 (no external domains)
```

### 4. Test CSP
```js
// In browser console, try loading external script:
const script = document.createElement('script');
script.src = 'https://www.google-analytics.com/analytics.js';
document.body.appendChild(script);

// Expected: CSP violation error in console
// "Refused to load the script ... violates the following Content Security Policy directive: 'script-src 'self''"
```

---

## Audit Artifacts

### Files Reviewed
- ✅ `frontend/app/layout.tsx`
- ✅ `frontend/package.json`
- ✅ `frontend/next.config.ts`
- ✅ All `frontend/app/**/*.tsx` components
- ✅ All `frontend/app/api/**/*.ts` API routes

### Screenshots
- Cookie storage panel (empty): N/A
- localStorage panel: Shows `theme` key only
- Network requests: All to `self` origin

### Testing Environment
- **Browser**: Chrome 131.0
- **OS**: Linux
- **Node**: v24.x
- **Next.js**: v16.2.12
- **Date**: 2026-07-29

---

## Conclusion

**Recommendation**: ✅ **No action required at this time**

The Sanctifier dashboard is **fully compliant** with GDPR/ePrivacy regulations as it:
- Sets zero cookies
- Uses zero third-party trackers
- Only stores essential UI preferences locally (theme)
- Has a strict CSP preventing unauthorized tracking

**Future Action**: If analytics/error tracking is added via #1148 or similar, revisit this audit and implement the provided `CookieConsentBanner` component.

---

## References

- **GDPR**: [Regulation (EU) 2016/679](https://gdpr-info.eu/)
- **ePrivacy Directive**: [Directive 2002/58/EC](https://eur-lex.europa.eu/legal-content/EN/ALL/?uri=CELEX:32002L0058)
- **ICO Guidance**: [Cookies and similar technologies](https://ico.org.uk/for-organisations/direct-marketing-and-privacy-and-electronic-communications/guide-to-pecr/cookies-and-similar-technologies/)
- **CNIL Guidance**: [French Data Protection Authority - Cookies](https://www.cnil.fr/en/cookies-and-other-trackers)

---

**Document Owner**: Frontend Team  
**Next Review**: When #1148 (Sentry integration) or similar analytics tools are implemented  
**Related Issues**: #1148 (error tracking), #1158 (privacy policy), #1159 (GDPR compliance)

---

## Appendix: Cookie Consent Banner Component

A ready-to-use consent banner component has been created at:
- `frontend/app/components/CookieConsentBanner.tsx`

**Features**:
- ✅ GDPR/ePrivacy compliant
- ✅ Blocks analytics until consent given
- ✅ Persistent consent storage (localStorage)
- ✅ Opt-out mechanism
- ✅ Accessible (keyboard navigation, ARIA labels)
- ✅ Responsive design
- ✅ No dependencies (vanilla React)

**Usage**: Import and render at the bottom of `layout.tsx` when analytics are added.
