class MermaidHeightAdjuster {
    constructor() {
        this.scale = 100;
        this.renderedTabs = new Set();
    }

    setupTabSwitching() {
        document.querySelectorAll('.nav-tab').forEach(tab => {
            tab.addEventListener('click', async () => {
                document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(tc => tc.classList.remove('active'));

                tab.classList.add('active');
                const targetId = tab.getAttribute('data-tab');
                const target = document.getElementById(targetId);
                if (target) {
                    target.classList.add('active');
                    await this.renderAndScale(target);
                }
            });
        });
    }

    setupScaleControls() {
        document.addEventListener('click', (e) => {
            const btn = e.target.closest('[data-scale-delta]');
            if (btn) {
                const delta = parseInt(btn.getAttribute('data-scale-delta'), 10);
                this.changeScale(delta);
                return;
            }
            const resetBtn = e.target.closest('[data-reset-scale]');
            if (resetBtn) {
                this.resetScale();
                return;
            }
        });
    }

    async renderAndScale(container) {
        const tabId = container.id;
        const diagramEl = container.querySelector('.mermaid');
        if (!diagramEl) return;

        if (this.renderedTabs.has(tabId)) {
            this.adjustDiagram(container);
            this.updateScaleDisplay();
            return;
        }

        const originalDef = diagramEl.dataset.originalDefinition || diagramEl.textContent.trim();
        diagramEl.textContent = originalDef;

        try {
            await mermaidReady;
            await mermaid.run({ nodes: [diagramEl] });
            this.renderedTabs.add(tabId);
            this.adjustDiagram(container);
            this.updateScaleDisplay();
        } catch (err) {
            console.error('Mermaid render error for', tabId, err);
            diagramEl.textContent = 'Error rendering diagram.';
        }
    }

    adjustDiagram(container) {
        const diagramContainer = container.querySelector('.diagram-container');
        if (!diagramContainer) return;
        const svg = diagramContainer.querySelector('svg');
        if (!svg) return;

        const bbox = svg.getBBox();
        const scaleFactor = this.scale / 100;
        const padding = 40;

        svg.setAttribute('width', (bbox.width * scaleFactor + padding) + 'px');
        svg.setAttribute('height', (bbox.height * scaleFactor + padding) + 'px');
        svg.style.transform = `scale(${scaleFactor})`;
        svg.style.transformOrigin = 'top left';
    }

    changeScale(delta) {
        this.scale = Math.min(200, Math.max(50, this.scale + delta));
        this.updateScaleDisplay();
        document.querySelectorAll('.tab-content.active').forEach(tc => this.adjustDiagram(tc));
    }

    resetScale() {
        this.scale = 100;
        this.updateScaleDisplay();
        document.querySelectorAll('.tab-content.active').forEach(tc => this.adjustDiagram(tc));
    }

    updateScaleDisplay() {
        document.querySelectorAll('.scale-display').forEach(el => {
            el.textContent = this.scale + '%';
        });
    }
}
