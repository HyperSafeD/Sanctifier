const API_URL = "https://api.sanctifier.dev/analyze";
const sourceInput = document.getElementById("source-input");
const analyzeButton = document.getElementById("analyze-btn");
const status = document.getElementById("status");
const result = document.getElementById("result");

function resetResult() {
  result.textContent = "";
  result.hidden = true;
}

analyzeButton.addEventListener("click", async () => {
  resetResult();

  const source = sourceInput.value.trim();
  if (!source) {
    status.textContent = "Paste Soroban source before analyzing.";
    return;
  }

  status.textContent = "Sending source to Sanctifier hosted API...";
  analyzeButton.disabled = true;

  try {
    const response = await fetch(API_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ source }),
    });

    if (!response.ok) {
      const text = await response.text();
      status.textContent = `API error: ${response.status} ${response.statusText}`;
      result.textContent = text;
      result.hidden = false;
      return;
    }

    const json = await response.json();
    status.textContent = "Analysis complete. Review the results below.";
    result.textContent = JSON.stringify(json, null, 2);
    result.hidden = false;
  } catch (error) {
    status.textContent = `Network error while calling hosted API: ${error}`;
  } finally {
    analyzeButton.disabled = false;
  }
});
