/// <reference path="./env.d.ts" />
import type { Theme } from 'vitepress'
import DefaultTheme, { VPTeamMembers } from 'vitepress/theme'
import './style.css'
import Layout from './Layout.vue'
import GlowCard from './components/GlowCard.vue'
import TerminalBlock from './components/TerminalBlock.vue'
import StatCounter from './components/StatCounter.vue'
import TyrusFooter from './components/TyrusFooter.vue'
import SignatureBlock from './components/SignatureBlock.vue'

const theme: Theme = {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    app.component('VPTeamMembers', VPTeamMembers),
    app.component('GlowCard', GlowCard)
    app.component('TerminalBlock', TerminalBlock)
    app.component('StatCounter', StatCounter)
    app.component('TyrusFooter', TyrusFooter)
    app.component('SignatureBlock', SignatureBlock)
  }
}

export default theme
