import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import {
  CookieConsentBanner,
  hasConsent,
  CookiePreferenceLink,
} from "../CookieConsentBanner";

describe("CookieConsentBanner", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("should render banner when no consent is saved", () => {
    render(<CookieConsentBanner />);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/Cookie Preferences/i)).toBeInTheDocument();
    expect(screen.getByText(/Accept All/i)).toBeInTheDocument();
    expect(screen.getByText(/Reject All/i)).toBeInTheDocument();
    expect(screen.getByText(/Customize/i)).toBeInTheDocument();
  });

  it("should not render banner when consent is already saved", () => {
    localStorage.setItem(
      "sanctifier-cookie-consent",
      JSON.stringify({
        necessary: true,
        analytics: false,
        timestamp: new Date().toISOString(),
      })
    );

    render(<CookieConsentBanner />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("should save preferences when clicking Accept All", () => {
    const reloadMock = vi.fn();
    Object.defineProperty(window, "location", {
      value: { reload: reloadMock },
      writable: true,
    });

    render(<CookieConsentBanner />);

    const acceptButton = screen.getByText(/Accept All/i);
    fireEvent.click(acceptButton);

    const saved = localStorage.getItem("sanctifier-cookie-consent");
    expect(saved).toBeTruthy();

    const prefs = JSON.parse(saved!);
    expect(prefs.necessary).toBe(true);
    expect(prefs.analytics).toBe(true);
    expect(prefs.timestamp).toBeTruthy();
    expect(reloadMock).toHaveBeenCalledTimes(1);
  });

  it("should save preferences when clicking Reject All", () => {
    render(<CookieConsentBanner />);

    const rejectButton = screen.getByText(/Reject All/i);
    fireEvent.click(rejectButton);

    const saved = localStorage.getItem("sanctifier-cookie-consent");
    expect(saved).toBeTruthy();

    const prefs = JSON.parse(saved!);
    expect(prefs.necessary).toBe(true);
    expect(prefs.analytics).toBe(false);
    expect(prefs.timestamp).toBeTruthy();
  });

  it("should show settings when clicking Customize", () => {
    render(<CookieConsentBanner />);

    const customizeButton = screen.getByText(/Customize/i);
    fireEvent.click(customizeButton);

    expect(screen.getByRole("heading", { name: /Essential Cookies/i })).toBeInTheDocument();
    expect(screen.getByText(/Analytics & Performance/i)).toBeInTheDocument();
    expect(screen.getByText(/Save Preferences/i)).toBeInTheDocument();
  });

  it("should toggle analytics checkbox in settings", () => {
    render(<CookieConsentBanner />);

    const customizeButton = screen.getByText(/Customize/i);
    fireEvent.click(customizeButton);

    const analyticsCheckbox = screen.getByLabelText(/Toggle analytics cookies/i);
    expect(analyticsCheckbox).not.toBeChecked();

    fireEvent.click(analyticsCheckbox);
    expect(analyticsCheckbox).toBeChecked();

    fireEvent.click(analyticsCheckbox);
    expect(analyticsCheckbox).not.toBeChecked();
  });

  it("should not allow disabling essential cookies", () => {
    render(<CookieConsentBanner />);

    const customizeButton = screen.getByText(/Customize/i);
    fireEvent.click(customizeButton);

    const essentialCheckbox = screen.getByLabelText(/Essential cookies/i);
    expect(essentialCheckbox).toBeChecked();
    expect(essentialCheckbox).toBeDisabled();
  });

  it("should save custom preferences", () => {
    const reloadMock = vi.fn();
    Object.defineProperty(window, "location", {
      value: { reload: reloadMock },
      writable: true,
    });

    render(<CookieConsentBanner />);

    const customizeButton = screen.getByText(/Customize/i);
    fireEvent.click(customizeButton);

    const analyticsCheckbox = screen.getByLabelText(/Toggle analytics cookies/i);
    fireEvent.click(analyticsCheckbox); // Enable analytics

    const saveButton = screen.getByText(/Save Preferences/i);
    fireEvent.click(saveButton);

    const saved = localStorage.getItem("sanctifier-cookie-consent");
    const prefs = JSON.parse(saved!);

    expect(prefs.necessary).toBe(true);
    expect(prefs.analytics).toBe(true);
    expect(reloadMock).toHaveBeenCalledTimes(1);
  });

  it("should close settings when clicking Cancel", () => {
    render(<CookieConsentBanner />);

    const customizeButton = screen.getByText(/Customize/i);
    fireEvent.click(customizeButton);

    expect(screen.getByText(/Save Preferences/i)).toBeInTheDocument();

    const cancelButton = screen.getByText(/Cancel/i);
    fireEvent.click(cancelButton);

    expect(screen.queryByText(/Save Preferences/i)).not.toBeInTheDocument();
    expect(screen.getByText(/Accept All/i)).toBeInTheDocument();
  });
});

describe("hasConsent", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("should return true for necessary category", () => {
    expect(hasConsent("necessary")).toBe(true);
  });

  it("should return false for analytics when no consent saved", () => {
    expect(hasConsent("analytics")).toBe(false);
  });

  it("should return true for analytics when consent is given", () => {
    localStorage.setItem(
      "sanctifier-cookie-consent",
      JSON.stringify({
        necessary: true,
        analytics: true,
        timestamp: new Date().toISOString(),
      })
    );

    expect(hasConsent("analytics")).toBe(true);
  });

  it("should return false for analytics when consent is denied", () => {
    localStorage.setItem(
      "sanctifier-cookie-consent",
      JSON.stringify({
        necessary: true,
        analytics: false,
        timestamp: new Date().toISOString(),
      })
    );

    expect(hasConsent("analytics")).toBe(false);
  });

  it("should handle invalid JSON gracefully", () => {
    localStorage.setItem("sanctifier-cookie-consent", "invalid-json");

    expect(hasConsent("analytics")).toBe(false);
  });
});

describe("CookiePreferenceLink", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("should render preference link", () => {
    render(<CookiePreferenceLink />);

    const link = screen.getByText(/Cookie Preferences/i);
    expect(link).toBeInTheDocument();
  });

  it("should clear consent and reload when clicked", () => {
    const reloadMock = vi.fn();
    Object.defineProperty(window, "location", {
      value: { reload: reloadMock },
      writable: true,
    });

    localStorage.setItem(
      "sanctifier-cookie-consent",
      JSON.stringify({
        necessary: true,
        analytics: true,
        timestamp: new Date().toISOString(),
      })
    );

    render(<CookiePreferenceLink />);

    const link = screen.getByText(/Cookie Preferences/i);
    fireEvent.click(link);

    expect(localStorage.getItem("sanctifier-cookie-consent")).toBeNull();
    expect(reloadMock).toHaveBeenCalledTimes(1);
  });
});
