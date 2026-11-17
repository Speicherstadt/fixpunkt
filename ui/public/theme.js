// Hell oder dunkel vor dem ersten Zeichnen (Wahl unter Konto → Darstellung, src/lib/theme.ts).
// Vorbild: firsttris/kontext public/theme.js
(() => {
  let choice = null;
  try {
    choice = localStorage.getItem("fixpunkt.theme");
  } catch {
    // Speicher verweigert: dann entscheidet das Gerät
  }
  const dark =
    choice === "dark" || (choice !== "light" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.add(dark ? "dark" : "light");
})();
