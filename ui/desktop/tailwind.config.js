/** @type {import('tailwindcss').Config} */
export default {
  // Extend the web UI tailwind config
  presets: [require('../web/tailwind.config.js')],
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
    '../web/src/**/*.{js,ts,jsx,tsx}', // Include web UI components
  ],
}
