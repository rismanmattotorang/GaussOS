import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

const resources = {
  en: {
    translation: {
      // Navigation
      'nav.dashboard': 'Dashboard',
      'nav.simulations': 'Simulations',
      'nav.agents': 'Agents',
      'nav.spaces': 'Spaces',
      'nav.analytics': 'Analytics',
      'nav.settings': 'Settings',
      'nav.developer': 'Developer',
      'nav.apiExplorer': 'API Explorer',
      'nav.logout': 'Logout',

      // Auth
      'auth.login': 'Sign In',
      'auth.register': 'Create Account',
      'auth.email': 'Email',
      'auth.password': 'Password',
      'auth.confirmPassword': 'Confirm Password',
      'auth.forgotPassword': 'Forgot password?',
      'auth.noAccount': "Don't have an account?",
      'auth.hasAccount': 'Already have an account?',
      'auth.signUp': 'Sign up',
      'auth.signIn': 'Sign in',
      'auth.welcome': 'Welcome back',
      'auth.createAccount': 'Create your account',
      'auth.continueWith': 'Or continue with',

      // Dashboard
      'dashboard.title': 'Dashboard',
      'dashboard.welcome': 'Welcome to GaussTwin',
      'dashboard.activeSimulations': 'Active Simulations',
      'dashboard.totalAgents': 'Total Agents',
      'dashboard.cpuUsage': 'CPU Usage',
      'dashboard.memoryUsage': 'Memory Usage',
      'dashboard.gpuUsage': 'GPU Usage',
      'dashboard.recentActivity': 'Recent Activity',
      'dashboard.quickActions': 'Quick Actions',
      'dashboard.newSimulation': 'New Simulation',

      // Simulations
      'simulations.title': 'Simulations',
      'simulations.create': 'Create Simulation',
      'simulations.search': 'Search simulations...',
      'simulations.status.running': 'Running',
      'simulations.status.paused': 'Paused',
      'simulations.status.stopped': 'Stopped',
      'simulations.status.completed': 'Completed',
      'simulations.status.error': 'Error',
      'simulations.actions.start': 'Start',
      'simulations.actions.pause': 'Pause',
      'simulations.actions.stop': 'Stop',
      'simulations.actions.restart': 'Restart',
      'simulations.actions.delete': 'Delete',
      'simulations.metrics.agents': 'Agents',
      'simulations.metrics.steps': 'Steps',
      'simulations.metrics.time': 'Time',

      // Agents
      'agents.title': 'Agent Catalog',
      'agents.create': 'Create Agent',
      'agents.types.bdi': 'BDI Agent',
      'agents.types.cognitive': 'Cognitive Agent',
      'agents.types.reactive': 'Reactive Agent',
      'agents.types.custom': 'Custom Agent',

      // Spaces
      'spaces.title': 'Spaces',
      'spaces.types.grid': 'Grid Space',
      'spaces.types.continuous': 'Continuous Space',
      'spaces.types.graph': 'Graph Space',

      // Analytics
      'analytics.title': 'Analytics',
      'analytics.predictive': 'Predictive Analytics',
      'analytics.descriptive': 'Descriptive Analytics',
      'analytics.anomalies': 'Anomaly Detection',

      // Settings
      'settings.title': 'Settings',
      'settings.profile': 'Profile',
      'settings.appearance': 'Appearance',
      'settings.notifications': 'Notifications',
      'settings.apiKeys': 'API Keys',
      'settings.theme.light': 'Light',
      'settings.theme.dark': 'Dark',
      'settings.theme.system': 'System',

      // Common
      'common.loading': 'Loading...',
      'common.error': 'Error',
      'common.success': 'Success',
      'common.save': 'Save',
      'common.cancel': 'Cancel',
      'common.delete': 'Delete',
      'common.edit': 'Edit',
      'common.create': 'Create',
      'common.search': 'Search',
      'common.filter': 'Filter',
      'common.sort': 'Sort',
      'common.export': 'Export',
      'common.import': 'Import',
      'common.refresh': 'Refresh',
      'common.noData': 'No data available',
      'common.confirm': 'Confirm',
      'common.back': 'Back',
      'common.next': 'Next',
      'common.submit': 'Submit',
    },
  },
}

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  })

export default i18n
