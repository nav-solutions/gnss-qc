use crate::prelude::QcReport;

impl QcReport {
    /// Generates CSS for this (autonomous) page
    pub(crate) fn page_css(&self) -> String {
        "
:root {
  --bg-color: #1e1e2f;
  --primary: #00bcd4;
  --secondary: #44475a;
  --text-color: #f8f8f2;
  --accent: #8be9fd;
  --card-bg: #2e2e3e;
}

body {
  margin: 0;
  font-family: 'Segoe UI', sans-serif;
  background-color: var(--bg-color);
  color: var(--text-color);
  display: flex;
  min-height: 100vh;
}

nav {
  width: 250px;
  background-color: var(--secondary);
  padding: 2rem 1rem;
  box-shadow: 2px 0 5px rgba(0, 0, 0, 0.3);
  display: flex;
  flex-direction: column;
  gap: 1rem;
  transition: transform 0.4s ease;
}

nav.hidden {
  transform: translateX(-100%);
}

nav h1 {
  font-size: 1.5rem;
  color: var(--accent);
  margin-bottom: 2rem;
}

nav a {
  color: var(--text-color);
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: 6px;
  transition: background 0.3s ease;
  cursor: pointer;
}

nav a:hover {
  background-color: var(--accent);
  color: #000;
}

.content {
  flex-grow: 1;
  padding: 2rem;
  display: flex;
  flex-direction: column;
}

.section {
  background-color: var(--card-bg);
  border-radius: 10px;
  padding: 1.5rem;
  margin-bottom: 2rem;
  box-shadow: 0 0 10px rgba(0,0,0,0.2);
  display: none;
}

.section.active {
  display: block;
}

.section h2 {
  margin-top: 0;
  color: var(--primary);
}

.plot-placeholder {
  background-color: #333;
  height: 300px;
  border: 2px dashed #555;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #999;
  border-radius: 8px;
  font-style: italic;
}

.tabs {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
}

.tab {
  padding: 0.5rem 1rem;
  background-color: var(--secondary);
  border-radius: 5px;
  cursor: pointer;
}

.tab:hover {
  background-color: var(--accent);
  color: #000;
}

.tab.active {
  background-color: var(--accent);
  color: #000;
  font-weight: bold;
}

.styled-table {
  width: 100%;
  border-collapse: collapse;
  margin: 1rem 0;
  font-size: 0.95rem;
  background-color: var(--card-bg);
  color: var(--text-color);
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 0 10px rgba(0, 0, 0, 0.25);
}

.styled-table thead {
  background-color: var(--secondary);
  text-align: left;
}

.styled-table th, .styled-table td {
  padding: 0.75rem 1rem;
}

.styled-table tbody tr {
  border-bottom: 1px solid #4444;
}

.styled-table tbody tr:hover {
  background-color: rgba(255, 255, 255, 0.05);
}

.styled-table tbody tr:last-of-type {
  border-bottom: none;
}

.content-section {
  display: none;
  margin-top: 1rem;
  padding: 1rem;
  background-color: var(--card-bg);
  border-radius: 8px;
  box-shadow: 0 0 10px rgba(0, 0, 0, 0.25);
}

.content-section active {
  display: block;
}
        "
        .to_string()
    }
}
