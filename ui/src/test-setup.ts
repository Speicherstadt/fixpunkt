import "@testing-library/dom";
import { overwriteGetLocale } from "@/paraglide/runtime";

// Die Tests prüfen deutsche Texte und Formate; jsdom meldet Englisch.
overwriteGetLocale(() => "de");
