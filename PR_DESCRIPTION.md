# PR: Implement 4 Contrib-Wave Issues (#779, #761, #793, #780)

## Summary
This PR resolves four open contrib-wave issues that add critical security scanning features to the Sanctifier dashboard:
1. **#779**: Comparison view with side-by-side diff for analyzing two security reports
2. **#761**: Extended arithmetic rule to detect division and modulo-by-zero panic paths
3. **#793**: Public REST API endpoint for one-shot analysis with auth and rate limiting
4. **#780**: Browser extension popup for quick preview of recent findings

## Detailed Changes

### Issue #779: Frontend - Comparison View (Side-by-Side Diff)
**Files Modified/Created:**
- `frontend/app/types.ts`: Added `DiffStatus`, `DiffFinding`, and `ReportDiff` types
- `frontend/app/components/ComparisonView.tsx` (NEW): React component for displaying side-by-side diff with:
  - Summary cards showing baseline vs current findings
  - Filters for status and sort options (by status, severity, code)
  - Severity-aware color coding (regressions highlighted in red)
  - Export diff as JSON functionality
- `frontend/app/lib/diff.ts` (NEW): Core diff computation logic:
  - `computeReportDiff()`: Compares two reports and categorizes findings (added, removed, unchanged, severity_changed)
  - `exportDiffAsJson()`: Exports merged diff with schema version and summary
- `frontend/app/dashboard/page.tsx`: Integrated ComparisonView component with upload UI for baseline report

**Features:**
✅ Upload two reports → side-by-side findings list with deltas highlighted  
✅ Severity-aware coloring (regressions in red, fixes in green)  
✅ Export merged diff as JSON  
✅ Filter by status and sort by severity/code  

**Testing:**
- Supports uploading baseline JSON and comparing with current report
- Fingerprinting prevents false positives from line number shifts
- Properly categorizes severity changes

---

### Issue #761: Engine - Extend Arithmetic Rule S003 for Division/Modulo
**Files Modified:**
- `tooling/sanctifier-core/src/rules/arithmetic_overflow.rs`: Extended to detect and suggest fixes for:
  - Division (`/`) operations with non-constant divisors → `checked_div()`
  - Modulo (`%`) operations with non-constant divisors → `checked_rem()`
  - Division assignment (`/=`) → `checked_div()` with error handling
  - Modulo assignment (`%=`) → `checked_rem()` with error handling

**Implementation Details:**
- `is_non_constant_divisor()` function: Filters out compile-time constant divisors to reduce false positives
- `classify_op()` method: Maps binary operators to appropriate checked method suggestions
- Proper message formatting for each operation type

**Test Coverage:**
- Flagged standard arithmetic operations (+, -, *, /)
- Flagged custom math methods (mul_div, fixed_point_mul, etc.)
- Correct handling of constant vs non-constant operands
- Proper skipping of test modules and array index expressions

**Acceptance Criteria Met:**
✅ Extend rule to flag `/` and `%` on non-constant divisors  
✅ Suggestion templates use `checked_div`/`checked_rem`  

---

### Issue #793: Integrations - Hosted REST API (Rate-Limited)
**Files Created:**
- `frontend/app/api/v1/analyze/route.ts` (NEW): Public REST API endpoint with:
  - **Authentication**: x-api-key header (also supports Authorization: Bearer)
  - **Rate limiting**: 5 requests/minute per key, with Retry-After headers
  - **File uploads**: Multipart form data (`.rs` files up to 250 KB) or JSON request body
  - **Output formats**: JSON (default) + SARIF 2.1 for CI/CD integration
  - **Validation**: Soroban contract detection (requires soroban-sdk import)
  - **Error handling**: Comprehensive HTTP status codes (400, 401, 413, 422, 429, 504)
  
- `docs/api-pricing-limits.md` (NEW): Complete API documentation covering:
  - Rate limit tiers (Free: 5 req/min, Pro: 20 req/min coming soon)
  - File size limits (250 KB)
  - Execution timeouts (30 seconds)
  - Supported formats and response examples
  - Error codes and best practices
  - Integration examples with curl and GitHub Actions

**Features:**
✅ Public POST `/v1/analyze` endpoint (auth via API key)  
✅ Returns JSON + SARIF format  
✅ Pricing/limits documentation  
✅ Deployed via existing infra (Next.js API routes)  

**API Endpoints:**
```
POST /api/v1/analyze?format=json|sarif
GET /api/v1/analyze (service info)
```

**Authentication Methods:**
```
Headers:
  x-api-key: sk_test_abc123
  OR
  Authorization: Bearer sk_test_abc123
```

---

### Issue #780: Frontend - Browser Extension (Read-Only Finding Viewer)
**Files Created/Modified:**
- `browser-extension/manifest.json`: MV3 manifest with:
  - Service worker for background polling (every 5 minutes)
  - Host permissions for localhost:3000 and 127.0.0.1:9100
  - Action/popup configuration
  - Firefox support (strict_min_version: 109.0)

- `browser-extension/popup.html` (COMPLETED): Styled popup UI featuring:
  - Status indicator (connected/disconnected/loading)
  - Badge showing finding count
  - Scrollable list of last 10 findings
  - Severity-aware color coding
  - "Open Dashboard" footer link
  - Responsive design with dark mode support

- `browser-extension/popup.js` (COMPLETED): Popup logic:
  - Fetches findings from `/api/recent-findings`
  - Displays findings with code, severity, title, location
  - Status indicator shows connection state
  - Auto-refresh every 30 seconds while popup is open
  - Deep link to dashboard on finding click

- `browser-extension/background.js` (COMPLETED): Background service worker:
  - Alarm-based polling every 5 minutes
  - Caches findings in chrome.storage.local
  - Handles polling errors gracefully
  - Supports manual refresh from popup

- `browser-extension/build-icons.js`: Icon generation script supporting:
  - sharp (npm package)
  - ImageMagick (convert command)
  - librsvg (rsvg-convert)

- `browser-extension/README.md` (COMPLETED): Setup and usage documentation

**Features:**
✅ Chrome/Firefox MV3 extension polling local sanctifier-serve instance  
✅ Popup shows last 10 findings  
✅ Click → open dashboard with deep link  

**Installation Steps:**
1. Chrome: `chrome://extensions` → Load unpacked → select `browser-extension/`
2. Firefox: `about:debugging` → Load Temporary Add-on → select `manifest.json`

**Configuration:**
- Dashboard URL: `http://localhost:3000` (configurable)
- Poll interval: 5 minutes (background) / 30 seconds (popup)
- Max findings displayed: 10

---

## Files Changed Summary

### New Files
| File | Lines | Purpose |
|------|-------|---------|
| `frontend/app/components/ComparisonView.tsx` | 275 | Side-by-side diff viewer component |
| `frontend/app/lib/diff.ts` | 150 | Report diff computation logic |
| `frontend/app/api/v1/analyze/route.ts` | 280 | Public REST API endpoint |
| `docs/api-pricing-limits.md` | 350 | API documentation |
| `browser-extension/manifest.json` | 20 | MV3 manifest |
| `browser-extension/popup.html` | 150 | Extension popup UI |
| `browser-extension/popup.js` | 120 | Popup logic |
| `browser-extension/background.js` | 50 | Background polling |
| `browser-extension/build-icons.js` | 65 | Icon generation |
| `browser-extension/README.md` | 80 | Extension documentation |

### Modified Files
| File | Changes |
|------|---------|
| `frontend/app/types.ts` | Added Diff* types |
| `frontend/app/dashboard/page.tsx` | Added diff tab + baseline upload UI |
| `frontend/app/lib/env.ts` | Added API env vars |
| `frontend/app/api/analyze/route.ts` | Integration with recent-findings |
| `tooling/sanctifier-core/src/rules/arithmetic_overflow.rs` | Division/modulo detection |
| `frontend/app/api/recent-findings/route.ts` | NEW: Findings cache endpoint |
| `frontend/app/lib/sarif.ts` | SARIF export functionality |

---

## Testing & Validation

### Frontend (Issue #779)
- [x] Upload two JSON reports
- [x] Verify comparison view renders correctly
- [x] Check severity coloring (red for regressions, green for fixes)
- [x] Test export JSON functionality
- [x] Verify status and severity filters work
- [x] Test sorting options

### Engine (Issue #761)
- [x] Division operator (/) with non-constant divisor detection
- [x] Modulo operator (%) with non-constant divisor detection
- [x] Verify suggestions use `checked_div()` and `checked_rem()`
- [x] Test assignment operators (/=, %=)
- [x] Ensure constant divisors don't trigger false positives

### API (Issue #793)
- [x] POST `/api/v1/analyze` with JSON body
- [x] POST `/api/v1/analyze` with multipart form-data
- [x] GET `/api/v1/analyze` (service info)
- [x] Verify x-api-key authentication
- [x] Test rate limiting (429 response)
- [x] Verify SARIF output format
- [x] Test file size validation
- [x] Ensure Soroban contract validation works

### Browser Extension (Issue #780)
- [x] Extension loads in Chrome
- [x] Extension loads in Firefox
- [x] Background polling works every 5 minutes
- [x] Popup fetches and displays findings
- [x] Severity coloring works
- [x] Click finding → opens dashboard
- [x] Status indicator shows connection state

---

## Backwards Compatibility
✅ All changes are additive and non-breaking
✅ Existing APIs remain unchanged
✅ New features are opt-in
✅ No database migrations required

## Dependencies
No new external dependencies added. Uses existing tech stack:
- Next.js API routes
- React components
- Rust syn/quote for AST analysis
- Chrome/Firefox Web APIs

## Security Considerations
- API key validation on all `/v1/analyze` requests
- Rate limiting prevents abuse (5 req/min default)
- File size limits prevent resource exhaustion (250 KB max)
- Execution timeout (30 seconds) prevents hanging processes
- Soroban contract validation ensures only intended targets
- CORS headers properly configured for API requests

## Performance Impact
- Diff computation: O(n) where n = number of findings (cached)
- Browser extension polling: 5-minute background interval (minimal CPU)
- REST API: Execution limited to 30 seconds with timeout fallback

## Documentation
✅ Inline code comments throughout
✅ API pricing/limits documentation
✅ Browser extension setup guide
✅ Example API requests with curl
✅ SARIF integration examples

---

## Related Issues
- Closes #779 (Comparison view)
- Closes #761 (Arithmetic rule extension)
- Closes #793 (REST API)
- Closes #780 (Browser extension)

## PR Checklist
- [x] Code follows project style conventions
- [x] Changes are well-commented
- [x] No breaking changes to existing APIs
- [x] New features have documentation
- [x] Error handling is comprehensive
- [x] All features are production-ready
- [x] Backwards compatible

---

**Reviewers:** Please verify each issue is fully addressed according to acceptance criteria.
