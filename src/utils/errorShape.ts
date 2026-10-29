/**
 * An error with its line number, frame and other numbers blurred, so the same thing happening
 * six hundred times collapses to one shape. The game writes the same failure once per frame,
 * which is why a log of a thousand error lines is usually a handful of problems.
 */
export const errorShape = (line: string): string =>
  line
    .replace(/^(ERROR|WARN)\s*:\s*\w+\s*f:\d+\s*(at [\w.]+\s*)?>?\s*/, '')
    .replace(/\d+/g, 'N')
    .trim()
    .slice(0, 90);
