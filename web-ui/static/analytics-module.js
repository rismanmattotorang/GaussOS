// GaussOS Premium Analytics Module - Enhanced Chart.js Version with Performance Optimizations
// Advanced real-time analytics with professional UI/UX and accessibility features

class GaussOSAnalytics {
    constructor() {
        this.updateInterval = null;
        this.charts = new Map();
        this.chartConfigs = new Map();
        this.animationSettings = {
            duration: 1200,
            easing: 'easeInOutCubic',
            delay: 100
        };
        this.colorSchemes = {
            primary: ['#667eea', '#764ba2', '#f093fb', '#f5576c', '#4facfe', '#00f2fe', '#43e97b'],
            gradients: [
                'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
                'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)',
                'linear-gradient(135deg, #43e97b 0%, #38f9d7 100%)',
                'linear-gradient(135deg, #ffecd2 0%, #fcb69f 100%)',
                'linear-gradient(135deg, #a8edea 0%, #fed6e3 100%)'
            ]
        };
        this.performance = window.GaussOSPerformance || { mark: () => {}, measure: () => {} };
        this.init();
    }

    init() {
        console.log('🔍 Initializing GaussOS Premium Analytics with Chart.js...');
        this.performance.mark('analytics-init-start');
        
        // Enhanced Chart.js availability check
        if (typeof window.Chart === 'undefined') {
            console.warn('Chart.js not available, attempting dynamic load...');
            this.loadChartJSDynamically();
            return;
        }
        
        console.log('✅ Chart.js available, version:', window.Chart?.version || 'unknown');
        
        // Configure Chart.js defaults for premium appearance
        this.configureChartDefaults();
        
        // Setup with delay for better UX
        setTimeout(() => {
            this.setupAnalyticsCharts();
            this.setupAnalyticsControls();
            this.setupResizeHandlers();
            this.startAnalyticsUpdates();
            this.performance.measure('analytics-init-complete', 'analytics-init-start');
        }, 200);
    }

    configureChartDefaults() {
        if (!window.Chart) return;
        
        // Enhanced default configuration
        const defaults = window.Chart.defaults;
        
        defaults.font = {
            family: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
            size: 12,
            weight: '500'
        };
        
        defaults.color = '#ffffff';
        defaults.backgroundColor = 'rgba(102, 126, 234, 0.1)';
        
        // Plugin defaults
        defaults.plugins.legend = {
            ...defaults.plugins.legend,
            labels: {
                usePointStyle: true,
                padding: 20,
                font: { weight: '600' },
                generateLabels: (chart) => {
                    const original = Chart.defaults.plugins.legend.labels.generateLabels;
                    const labels = original.call(this, chart);
                    
                    labels.forEach(label => {
                        label.borderRadius = 4;
                    });
                    
                    return labels;
                }
            }
        };
        
        defaults.plugins.tooltip = {
            ...defaults.plugins.tooltip,
            backgroundColor: 'rgba(15, 23, 42, 0.95)',
            titleColor: '#ffffff',
            bodyColor: '#cbd5e1',
            borderColor: 'rgba(102, 126, 234, 0.5)',
            borderWidth: 1,
            cornerRadius: 12,
            padding: 12,
            displayColors: true,
            titleFont: { weight: '700', size: 14 },
            bodyFont: { weight: '500', size: 13 },
            animation: {
                duration: 200,
                easing: 'easeOutCubic'
            },
            callbacks: {
                title: (context) => {
                    return context[0].label || 'Data Point';
                },
                label: (context) => {
                    let label = context.dataset.label || '';
                    if (label) label += ': ';
                    
                    const value = context.parsed.y ?? context.parsed;
                    if (typeof value === 'number') {
                        if (value > 1000000) {
                            label += (value / 1000000).toFixed(1) + 'M';
                        } else if (value > 1000) {
                            label += (value / 1000).toFixed(1) + 'K';
                        } else {
                            label += value.toLocaleString();
                        }
                    } else {
                        label += value;
                    }
                    
                    return label;
                }
            }
        };
        
        // Animation defaults
        defaults.animation = {
            duration: this.animationSettings.duration,
            easing: this.animationSettings.easing,
            delay: (context) => {
                let delay = 0;
                if (context.type === 'data' && context.mode === 'default') {
                    delay = context.dataIndex * 50 + context.datasetIndex * 100;
                }
                return Math.min(delay, 1000);
            }
        };
        
        // Responsive defaults
        defaults.responsive = true;
        defaults.maintainAspectRatio = false;
        
        // Interaction defaults
        defaults.interaction = {
            intersect: false,
            mode: 'index'
        };
    }

    loadChartJSDynamically() {
        const script = document.createElement('script');
        script.src = 'https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.js';
        script.onload = () => {
            console.log('✅ Chart.js loaded dynamically');
            this.configureChartDefaults();
            setTimeout(() => this.setupAnalyticsCharts(), 300);
        };
        script.onerror = () => {
            console.error('❌ Failed to load Chart.js dynamically');
            this.setupFallbackCharts();
        };
        document.head.appendChild(script);
    }

    setupAnalyticsCharts() {
        this.performance.mark('charts-setup-start');
        console.log('📊 Setting up premium analytics charts...');
        
        const chartSetupPromises = [
            this.createMemoryDistributionChart(),
            this.createPerformanceTimelineChart(),
            this.createAgentEfficiencyChart(),
            this.createSystemHealthGauge(),
            this.createThroughputHeatmap(),
            this.createMemoryOperationsFlow()
        ];
        
        Promise.all(chartSetupPromises).then(() => {
            this.performance.measure('charts-setup-complete', 'charts-setup-start');
            console.log('✅ All premium charts initialized successfully');
        }).catch(error => {
            console.error('❌ Error setting up charts:', error);
            this.setupFallbackCharts();
        });
    }

    async createMemoryDistributionChart() {
        const container = document.getElementById('memory-distribution-chart');
        if (!container) {
            console.warn('Memory distribution chart container not found');
            return;
        }

        const data = [
            { id: 'Semantic Memory', value: 28, color: '#667eea' },
            { id: 'Episodic Memory', value: 22, color: '#764ba2' },
            { id: 'Procedural Memory', value: 18, color: '#f093fb' },
            { id: 'Working Memory', value: 15, color: '#f5576c' },
            { id: 'Cached Memory', value: 12, color: '#4facfe' },
            { id: 'System Memory', value: 5, color: '#00f2fe' }
        ];

        if (typeof window.Chart === 'undefined') {
            this.createFallbackChart(container, 'Memory Distribution', '🧠', data);
            return;
        }

        try {
            // Create canvas with enhanced setup
            const canvas = this.createCanvas(container);
            
            // Enhanced Doughnut Chart with animations
            const chart = new window.Chart(canvas, {
                type: 'doughnut',
                data: {
                    labels: data.map(item => item.id),
                    datasets: [{
                        data: data.map(item => item.value),
                        backgroundColor: data.map(item => item.color),
                        borderWidth: 4,
                        borderColor: '#0f172a',
                        hoverBorderWidth: 6,
                        hoverBorderColor: '#ffffff'
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            position: 'bottom',
                            labels: {
                                padding: 25,
                                usePointStyle: true,
                                pointStyle: 'circle',
                                font: { 
                                    size: 13,
                                    weight: '600' 
                                }
                            }
                        },
                        tooltip: {
                            callbacks: {
                                label: (context) => {
                                    const total = context.dataset.data.reduce((a, b) => a + b, 0);
                                    const percentage = ((context.parsed / total) * 100).toFixed(1);
                                    return `${context.label}: ${context.parsed}% (${percentage}% of total)`;
                                }
                            }
                        }
                    },
                    cutout: '65%',
                    animation: {
                        animateRotate: true,
                        animateScale: true,
                        duration: 1500,
                        easing: 'easeOutBounce'
                    },
                    hover: {
                        animationDuration: 300
                    }
                }
            });

            this.charts.set('memory-distribution', chart);
            this.addChartInteractions(chart, 'Memory Distribution Analysis');
            console.log('✅ Memory distribution chart created');
            
        } catch (error) {
            console.error('Failed to create memory distribution chart:', error);
            this.createFallbackChart(container, 'Memory Distribution', '🧠', data);
        }
    }

    async createPerformanceTimelineChart() {
        const container = document.getElementById('performance-timeline-chart');
        if (!container) {
            console.warn('Performance timeline chart container not found');
            return;
        }

        const data = {
            cpu: [15, 25, 35, 45, 30, 20, 18, 22, 28, 35, 40, 25],
            memory: [45, 55, 65, 75, 70, 60, 58, 62, 68, 72, 78, 65],
            requests: [1200, 1800, 2200, 2800, 2400, 1600, 1400, 1800, 2100, 2600, 3000, 2200],
            labels: ['00:00', '02:00', '04:00', '06:00', '08:00', '10:00', '12:00', '14:00', '16:00', '18:00', '20:00', '22:00']
        };

        if (typeof window.Chart === 'undefined') {
            this.createFallbackChart(container, 'Performance Timeline', '📈', data);
            return;
        }

        try {
            const canvas = this.createCanvas(container);
            
            const chart = new window.Chart(canvas, {
                type: 'line',
                data: {
                    labels: data.labels,
                    datasets: [{
                        label: 'CPU Usage (%)',
                        data: data.cpu,
                        borderColor: '#667eea',
                        backgroundColor: 'rgba(102, 126, 234, 0.1)',
                        borderWidth: 3,
                        fill: true,
                        tension: 0.4,
                        pointBackgroundColor: '#667eea',
                        pointBorderColor: '#ffffff',
                        pointBorderWidth: 2,
                        pointRadius: 6,
                        pointHoverRadius: 8
                    }, {
                        label: 'Memory Usage (%)',
                        data: data.memory,
                        borderColor: '#f093fb',
                        backgroundColor: 'rgba(240, 147, 251, 0.1)',
                        borderWidth: 3,
                        fill: true,
                        tension: 0.4,
                        pointBackgroundColor: '#f093fb',
                        pointBorderColor: '#ffffff',
                        pointBorderWidth: 2,
                        pointRadius: 6,
                        pointHoverRadius: 8
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            position: 'top',
                            labels: {
                                padding: 20,
                                usePointStyle: true
                            }
                        },
                        tooltip: {
                            mode: 'index',
                            intersect: false
                        }
                    },
                    scales: {
                        y: {
                            beginAtZero: true,
                            max: 100,
                            grid: {
                                color: 'rgba(255, 255, 255, 0.1)',
                                lineWidth: 1
                            },
                            ticks: {
                                callback: (value) => value + '%'
                            }
                        },
                        x: {
                            grid: {
                                display: false
                            },
                            ticks: {
                                maxRotation: 0
                            }
                        }
                    },
                    animation: {
                        duration: 1200,
                        easing: 'easeInOutQuart'
                    },
                    hover: {
                        mode: 'nearest',
                        intersect: false
                    }
                }
            });

            this.charts.set('performance-timeline', chart);
            this.addChartInteractions(chart, 'Performance Timeline');
            console.log('✅ Performance timeline chart created');
            
        } catch (error) {
            console.error('Failed to create performance timeline chart:', error);
            this.createFallbackChart(container, 'Performance Timeline', '📈', data);
        }
    }

    async createAgentEfficiencyChart() {
        const container = document.getElementById('agent-efficiency-chart');
        if (!container) {
            console.warn('Agent efficiency chart container not found');
            return;
        }

        const data = [
            { agent: 'Analytics Agent', efficiency: 92 },
            { agent: 'Memory Agent', efficiency: 88 },
            { agent: 'Graph Agent', efficiency: 85 },
            { agent: 'Query Agent', efficiency: 90 },
            { agent: 'Cache Agent', efficiency: 94 },
            { agent: 'Sync Agent', efficiency: 87 }
        ];

        if (typeof window.Chart === 'undefined') {
            this.createFallbackChart(container, 'Agent Efficiency', '🤖', data);
            return;
        }

        try {
            const canvas = this.createCanvas(container);
            
            const chart = new window.Chart(canvas, {
                type: 'bar',
                data: {
                    labels: data.map(item => item.agent),
                    datasets: [{
                        label: 'Efficiency (%)',
                        data: data.map(item => item.efficiency),
                        backgroundColor: data.map((_, index) => this.colorSchemes.primary[index % this.colorSchemes.primary.length]),
                        borderColor: data.map((_, index) => this.colorSchemes.primary[index % this.colorSchemes.primary.length]),
                        borderWidth: 2,
                        borderRadius: 8,
                        borderSkipped: false,
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            display: false
                        },
                        tooltip: {
                            callbacks: {
                                title: (context) => context[0].label.replace(' Agent', ''),
                                label: (context) => `Efficiency: ${context.parsed.y}%`
                            }
                        }
                    },
                    scales: {
                        y: {
                            beginAtZero: true,
                            max: 100,
                            grid: {
                                color: 'rgba(255, 255, 255, 0.1)'
                            },
                            ticks: {
                                callback: (value) => value + '%'
                            }
                        },
                        x: {
                            grid: {
                                display: false
                            },
                            ticks: {
                                maxRotation: 45,
                                font: { size: 11 }
                            }
                        }
                    },
                    animation: {
                        duration: 1000,
                        easing: 'easeOutBounce'
                    },
                    hover: {
                        animationDuration: 200
                    }
                }
            });

            this.charts.set('agent-efficiency', chart);
            this.addChartInteractions(chart, 'Agent Efficiency Analysis');
            console.log('✅ Agent efficiency chart created');
            
        } catch (error) {
            console.error('Failed to create agent efficiency chart:', error);
            this.createFallbackChart(container, 'Agent Efficiency', '🤖', data);
        }
    }

    createSystemHealthGauge() {
        const container = document.getElementById('system-health-gauge');
        if (!container) {
            console.warn('System health gauge container not found');
            return;
        }

        const healthData = {
            uptime: 99.97,
            availability: 99.95,
            performance: 94.2,
            reliability: 99.1,
            security: 98.5,
            efficiency: 96.8
        };

        // Create enhanced health gauge with animations
        container.innerHTML = `
            <div class="system-health-grid">
                ${Object.entries(healthData).map(([key, value]) => {
                    const color = this.getHealthColor(value);
                    return `
                        <div class="health-metric" data-value="${value}">
                            <div class="health-label">${key.charAt(0).toUpperCase() + key.slice(1)}</div>
                            <div class="health-value">${value}%</div>
                            <div class="health-bar">
                                <div class="health-fill" 
                                     style="width: 0%; background: ${color}; transition: width 2s cubic-bezier(0.4, 0, 0.2, 1);">
                                </div>
                            </div>
                        </div>
                    `;
                }).join('')}
            </div>
        `;

        // Animate the gauges
        setTimeout(() => {
            container.querySelectorAll('.health-metric').forEach(metric => {
                const value = metric.dataset.value;
                const fill = metric.querySelector('.health-fill');
                if (fill) {
                    fill.style.width = value + '%';
                }
            });
        }, 500);

        console.log('✅ System health gauge created');
    }

    createThroughputHeatmap() {
        const container = document.getElementById('throughput-heatmap');
        if (!container) {
            console.warn('Throughput heatmap container not found');
            return;
        }

        // Generate sample heatmap data with realistic patterns
        const hours = ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00'];
        const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
        
        let heatmapHTML = `
            <div class="heatmap-container">
                <div class="heatmap-header">
                    <span>Throughput Heatmap (Requests/second)</span>
                    <div class="heatmap-legend">
                        <span>Low</span>
                        <div class="legend-gradient"></div>
                        <span>High</span>
                    </div>
                </div>
                <div class="heatmap-grid">
        `;
        
        // Header row
        heatmapHTML += '<div class="heatmap-cell header"></div>';
        hours.forEach(hour => {
            heatmapHTML += `<div class="heatmap-cell header">${hour}</div>`;
        });
        
        // Data rows with more realistic patterns
        days.forEach((day, dayIndex) => {
            heatmapHTML += `<div class="heatmap-cell header">${day}</div>`;
            hours.forEach((hour, hourIndex) => {
                // Create realistic traffic patterns
                let baseValue = 50;
                if (hourIndex >= 2 && hourIndex <= 4) baseValue += 30; // Peak hours
                if (dayIndex >= 5) baseValue *= 0.7; // Weekends
                
                const value = Math.floor(baseValue + Math.random() * 40);
                const intensity = Math.min(value / 120, 1);
                const color = `rgba(102, 126, 234, ${intensity})`;
                
                heatmapHTML += `
                    <div class="heatmap-cell heatmap-data" 
                         style="background-color: ${color};" 
                         title="${day} ${hour}: ${value} req/s"
                         data-value="${value}">
                        ${value}
                    </div>
                `;
            });
        });
        
        heatmapHTML += '</div></div>';
        container.innerHTML = heatmapHTML;

        // Add hover effects and animations
        setTimeout(() => {
            container.querySelectorAll('.heatmap-data').forEach(cell => {
                cell.addEventListener('mouseenter', (e) => {
                    e.target.style.transform = 'scale(1.1)';
                    e.target.style.zIndex = '10';
                    e.target.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.3)';
                });
                
                cell.addEventListener('mouseleave', (e) => {
                    e.target.style.transform = 'scale(1)';
                    e.target.style.zIndex = '1';
                    e.target.style.boxShadow = 'none';
                });
            });
        }, 100);

        console.log('✅ Throughput heatmap created');
    }

    createMemoryOperationsFlow() {
        const container = document.getElementById('memory-flow-chart');
        if (!container) {
            console.warn('Memory operations flow container not found');
            return;
        }

        const flowData = [
            { operation: 'Read Operations', count: 1450000, percentage: 42, color: '#667eea' },
            { operation: 'Write Operations', count: 980000, percentage: 28, color: '#f093fb' },
            { operation: 'Update Operations', count: 520000, percentage: 15, color: '#4facfe' },
            { operation: 'Delete Operations', count: 280000, percentage: 8, color: '#f5576c' },
            { operation: 'Index Operations', count: 180000, percentage: 5, color: '#43e97b' },
            { operation: 'Cache Operations', count: 70000, percentage: 2, color: '#00f2fe' }
        ];

        let flowHTML = `
            <div class="flow-container">
                <div class="flow-header">
                    <span>Memory Operations Flow (Per Second)</span>
                    <div class="flow-controls">
                        <button class="flow-toggle active" data-view="absolute">Count</button>
                        <button class="flow-toggle" data-view="percentage">%</button>
                    </div>
                </div>
                <div class="flow-items">
        `;
        
        flowData.forEach((item, index) => {
            flowHTML += `
                <div class="flow-item" data-index="${index}">
                    <div class="flow-operation">
                        <div class="operation-icon" style="background: ${item.color};">
                            <i class="fas fa-${this.getOperationIcon(item.operation)}"></i>
                        </div>
                        <span>${item.operation}</span>
                    </div>
                    <div class="flow-bar-container">
                        <div class="flow-bar">
                            <div class="flow-fill" 
                                 style="width: 0%; background: linear-gradient(90deg, ${item.color}, ${this.lightenColor(item.color, 20)});"
                                 data-width="${item.percentage}">
                            </div>
                        </div>
                        <div class="flow-value">
                            <span class="flow-count">${item.count.toLocaleString()}</span>
                            <span class="flow-percentage">(${item.percentage}%)</span>
                        </div>
                    </div>
                </div>
            `;
        });
        
        flowHTML += '</div></div>';
        container.innerHTML = flowHTML;

        // Animate the flow bars
        setTimeout(() => {
            container.querySelectorAll('.flow-fill').forEach(fill => {
                const width = fill.dataset.width;
                fill.style.width = width + '%';
            });
        }, 500);

        // Add toggle functionality
        container.querySelectorAll('.flow-toggle').forEach(toggle => {
            toggle.addEventListener('click', (e) => {
                const view = e.target.dataset.view;
                this.toggleFlowView(container, view);
                
                container.querySelectorAll('.flow-toggle').forEach(t => t.classList.remove('active'));
                e.target.classList.add('active');
            });
        });

        console.log('✅ Memory operations flow created');
    }

    getOperationIcon(operation) {
        const icons = {
            'Read Operations': 'search',
            'Write Operations': 'edit',
            'Update Operations': 'sync-alt',
            'Delete Operations': 'trash',
            'Index Operations': 'list',
            'Cache Operations': 'bolt'
        };
        return icons[operation] || 'cog';
    }

    lightenColor(color, percent) {
        const num = parseInt(color.replace('#', ''), 16);
        const amt = Math.round(2.55 * percent);
        const R = (num >> 16) + amt;
        const G = (num >> 8 & 0x00FF) + amt;
        const B = (num & 0x0000FF) + amt;
        return '#' + (0x1000000 + (R < 255 ? R < 1 ? 0 : R : 255) * 0x10000 +
            (G < 255 ? G < 1 ? 0 : G : 255) * 0x100 +
            (B < 255 ? B < 1 ? 0 : B : 255)).toString(16).slice(1);
    }

    toggleFlowView(container, view) {
        const items = container.querySelectorAll('.flow-item');
        items.forEach(item => {
            const countSpan = item.querySelector('.flow-count');
            const percentageSpan = item.querySelector('.flow-percentage');
            
            if (view === 'percentage') {
                countSpan.style.display = 'none';
                percentageSpan.style.display = 'inline';
                percentageSpan.textContent = percentageSpan.textContent.replace('(', '').replace(')', '');
            } else {
                countSpan.style.display = 'inline';
                percentageSpan.style.display = 'inline';
                percentageSpan.textContent = '(' + percentageSpan.textContent.replace('(', '').replace(')', '') + ')';
            }
        });
    }

    getHealthColor(value) {
        if (value >= 95) return 'linear-gradient(90deg, #10b981, #059669)';
        if (value >= 85) return 'linear-gradient(90deg, #f59e0b, #d97706)';
        if (value >= 70) return 'linear-gradient(90deg, #ef4444, #dc2626)';
        return 'linear-gradient(90deg, #6b7280, #4b5563)';
    }

    createCanvas(container) {
        const canvas = document.createElement('canvas');
        canvas.width = container.clientWidth || 400;
        canvas.height = container.clientHeight || 300;
        container.innerHTML = '';
        container.appendChild(canvas);
        return canvas;
    }

    addChartInteractions(chart, title) {
        const canvas = chart.canvas;
        
        // Add click handlers for data exploration
        canvas.addEventListener('click', (event) => {
            const points = chart.getElementsAtEventForMode(event, 'nearest', { intersect: true }, true);
            
            if (points.length) {
                const point = points[0];
                const datasetLabel = chart.data.datasets[point.datasetIndex].label;
                const label = chart.data.labels[point.index];
                const value = chart.data.datasets[point.datasetIndex].data[point.index];
                
                console.log(`📊 Chart interaction - ${title}:`, { datasetLabel, label, value });
                
                // Show detailed information
                this.showChartDetails(title, datasetLabel, label, value);
            }
        });
        
        // Add double-click for full-screen view
        canvas.addEventListener('dblclick', () => {
            this.toggleChartFullscreen(chart, title);
        });
    }

    showChartDetails(chartTitle, dataset, label, value) {
        // Create a modal or toast with detailed information
        const detail = document.createElement('div');
        detail.className = 'chart-detail-popup';
        detail.innerHTML = `
            <div class="detail-content">
                <h4>${chartTitle}</h4>
                <p><strong>Dataset:</strong> ${dataset}</p>
                <p><strong>Label:</strong> ${label}</p>
                <p><strong>Value:</strong> ${value}</p>
                <button onclick="this.parentElement.parentElement.remove()">Close</button>
            </div>
        `;
        
        detail.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: rgba(15, 23, 42, 0.95);
            color: white;
            padding: 20px;
            border-radius: 12px;
            border: 1px solid rgba(102, 126, 234, 0.5);
            z-index: 10000;
            backdrop-filter: blur(10px);
            animation: fadeIn 0.3s ease;
        `;
        
        document.body.appendChild(detail);
        
        setTimeout(() => detail.remove(), 5000);
    }

    toggleChartFullscreen(chart, title) {
        const canvas = chart.canvas;
        const container = canvas.parentElement;
        
        if (document.fullscreenElement) {
            document.exitFullscreen();
        } else {
            container.requestFullscreen().then(() => {
                chart.resize();
            }).catch(err => {
                console.warn('Fullscreen not supported:', err);
            });
        }
    }

    setupAnalyticsControls() {
        const timeframeButtons = document.querySelectorAll('.analytics-control-btn');
        timeframeButtons.forEach(button => {
            button.addEventListener('click', (e) => {
                timeframeButtons.forEach(btn => btn.classList.remove('active'));
                e.target.classList.add('active');
                const timeframe = e.target.getAttribute('data-timeframe');
                this.updateAnalyticsTimeframe(timeframe);
            });
        });
    }

    updateAnalyticsTimeframe(timeframe) {
        this.performance.mark(`timeframe-update-${timeframe}-start`);
        console.log(`⏱️ Updating analytics for timeframe: ${timeframe}`);
        
        // Show loading state
        this.showLoadingState();
        
        // Simulate data fetch and update
        setTimeout(() => {
            this.updateChartsWithNewTimeframe(timeframe);
            this.hideLoadingState();
            this.showNotification(`Analytics updated for ${timeframe} timeframe`, 'success');
            this.performance.measure(`timeframe-update-${timeframe}`, `timeframe-update-${timeframe}-start`);
        }, 800);
    }

    updateChartsWithNewTimeframe(timeframe) {
        // Update each chart with new data based on timeframe
        this.charts.forEach((chart, name) => {
            if (chart && typeof chart.update === 'function') {
                // Generate new data based on timeframe
                const newData = this.generateTimeframeData(name, timeframe);
                
                if (newData) {
                    chart.data = newData;
                    chart.update('active');
                }
            }
        });
    }

    generateTimeframeData(chartName, timeframe) {
        // Generate realistic data based on chart type and timeframe
        const multipliers = {
            '1h': { samples: 12, variance: 0.1 },
            '6h': { samples: 24, variance: 0.2 },
            '24h': { samples: 48, variance: 0.3 },
            '7d': { samples: 168, variance: 0.4 },
            '30d': { samples: 720, variance: 0.5 }
        };
        
        const config = multipliers[timeframe] || multipliers['1h'];
        
        switch (chartName) {
            case 'performance-timeline':
                return {
                    labels: Array.from({ length: config.samples }, (_, i) => 
                        this.generateTimeLabel(i, timeframe)
                    ),
                    datasets: [{
                        label: 'CPU Usage (%)',
                        data: Array.from({ length: config.samples }, () => 
                            Math.floor(Math.random() * 50 * config.variance + 15)
                        ),
                        borderColor: '#667eea',
                        backgroundColor: 'rgba(102, 126, 234, 0.1)',
                        borderWidth: 3,
                        fill: true,
                        tension: 0.4
                    }, {
                        label: 'Memory Usage (%)',
                        data: Array.from({ length: config.samples }, () => 
                            Math.floor(Math.random() * 40 * config.variance + 50)
                        ),
                        borderColor: '#f093fb',
                        backgroundColor: 'rgba(240, 147, 251, 0.1)',
                        borderWidth: 3,
                        fill: true,
                        tension: 0.4
                    }]
                };
            default:
                return null;
        }
    }

    generateTimeLabel(index, timeframe) {
        const now = new Date();
        let interval;
        
        switch (timeframe) {
            case '1h':
                interval = 5 * 60 * 1000; // 5 minutes
                break;
            case '6h':
                interval = 15 * 60 * 1000; // 15 minutes
                break;
            case '24h':
                interval = 30 * 60 * 1000; // 30 minutes
                break;
            case '7d':
                interval = 60 * 60 * 1000; // 1 hour
                break;
            case '30d':
                interval = 60 * 60 * 1000; // 1 hour
                break;
            default:
                interval = 5 * 60 * 1000;
        }
        
        const time = new Date(now.getTime() - (index * interval));
        return time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    showLoadingState() {
        document.querySelectorAll('.analytics-chart').forEach(chart => {
            const loader = document.createElement('div');
            loader.className = 'chart-loader';
            loader.innerHTML = `
                <div class="loader-spinner"></div>
                <p>Updating data...</p>
            `;
            loader.style.cssText = `
                position: absolute;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                background: rgba(15, 23, 42, 0.8);
                color: white;
                z-index: 100;
                backdrop-filter: blur(5px);
            `;
            
            chart.style.position = 'relative';
            chart.appendChild(loader);
        });
    }

    hideLoadingState() {
        document.querySelectorAll('.chart-loader').forEach(loader => {
            loader.remove();
        });
    }

    setupResizeHandlers() {
        let resizeTimeout;
        
        window.addEventListener('resize', () => {
            clearTimeout(resizeTimeout);
            resizeTimeout = setTimeout(() => {
                this.charts.forEach(chart => {
                    if (chart && typeof chart.resize === 'function') {
                        chart.resize();
                    }
                });
            }, 250);
        });
    }

    startAnalyticsUpdates() {
        // Update metrics every 10 seconds
        this.updateInterval = setInterval(() => {
            this.updateAnalyticsMetrics();
        }, 10000);
        
        // Update charts every 30 seconds
        setInterval(() => {
            this.updateChartsData();
        }, 30000);
    }

    updateAnalyticsMetrics() {
        const metricsUpdates = {
            'analytics-uptime': (99.9 + Math.random() * 0.09).toFixed(2) + '%',
            'analytics-requests': Math.floor(2000 + Math.random() * 1000).toLocaleString(),
            'analytics-response': Math.floor(30 + Math.random() * 30) + 'ms',
            'analytics-memory-ops': Math.floor(8000 + Math.random() * 4000).toLocaleString() + '+',
            'analytics-concurrent-users': Math.floor(400 + Math.random() * 100).toLocaleString(),
            'analytics-memory-usage': Math.floor(800 + Math.random() * 100) + 'MB',
            'analytics-cpu-usage': Math.floor(8 + Math.random() * 15) + '%',
            'analytics-error-rate': (Math.random() * 0.05).toFixed(3) + '%',
            'analytics-throughput': Math.floor(1500 + Math.random() * 500).toLocaleString()
        };
        
        Object.entries(metricsUpdates).forEach(([id, value]) => {
            const element = document.getElementById(id);
            if (element) {
                // Animate the value change
                element.style.transition = 'all 0.3s ease';
                element.style.transform = 'scale(1.05)';
                element.textContent = value;
                
                setTimeout(() => {
                    element.style.transform = 'scale(1)';
                }, 300);
            }
        });
    }

    updateChartsData() {
        this.charts.forEach((chart, name) => {
            if (chart && typeof chart.update === 'function') {
                // Add slight variations to existing data
                if (chart.data && chart.data.datasets) {
                    chart.data.datasets.forEach(dataset => {
                        if (dataset.data && Array.isArray(dataset.data)) {
                            dataset.data = dataset.data.map(value => {
                                const variation = (Math.random() - 0.5) * 0.1; // ±5% variation
                                return Math.max(0, Math.round(value * (1 + variation)));
                            });
                        }
                    });
                    
                    chart.update('none'); // Update without animation for real-time feel
                }
            }
        });
    }

    setupFallbackCharts() {
        console.log('📊 Setting up fallback charts...');
        
        const chartContainers = [
            'memory-distribution-chart',
            'performance-timeline-chart',
            'agent-efficiency-chart',
            'system-health-gauge',
            'throughput-heatmap',
            'memory-flow-chart'
        ];
        
        chartContainers.forEach(containerId => {
            const container = document.getElementById(containerId);
            if (container) {
                this.createFallbackChart(container, 'Analytics Chart', '📊', []);
            }
        });
    }

    createFallbackChart(container, title, icon, data) {
        console.log(`Creating fallback chart for ${title}`);
        
        const fallbackHTML = `
            <div class="fallback-chart">
                <div class="fallback-chart-icon">${icon}</div>
                <h4>${title}</h4>
                <p>${data.length > 0 ? 'Data Visualization' : 'Chart loading...'}</p>
                ${this.renderFallbackData(data)}
                <div class="fallback-status">
                    ${typeof window.Chart === 'undefined' ? 
                        '⏳ Chart.js loading...' : 
                        '✅ Interactive chart ready'
                    }
                </div>
            </div>
        `;
        
        container.innerHTML = fallbackHTML;
    }

    renderFallbackData(data) {
        if (!data || data.length === 0) return '<div class="fallback-chart-placeholder">No data available</div>';
        
        if (Array.isArray(data) && data[0] && typeof data[0] === 'object') {
            const keys = Object.keys(data[0]);
            
            if (keys.includes('value') && keys.includes('id')) {
                // Pie/Doughnut chart data
                return `
                    <div class="fallback-data-grid">
                        ${data.map(item => `
                            <div class="fallback-data-item">
                                <div class="color-indicator" style="background: ${item.color || '#667eea'};"></div>
                                <span class="data-label">${item.id}</span>
                                <span class="data-value">${item.value}%</span>
                            </div>
                        `).join('')}
                    </div>
                `;
            } else if (keys.includes('efficiency')) {
                // Bar chart data
                return `
                    <div class="fallback-bar-chart">
                        ${data.map(item => `
                            <div class="fallback-bar-item">
                                <div class="bar-label">${item.agent}</div>
                                <div class="bar-container">
                                    <div class="bar-fill" style="width: ${item.efficiency}%; background: linear-gradient(90deg, #667eea, #764ba2);"></div>
                                </div>
                                <div class="bar-value">${item.efficiency}%</div>
                            </div>
                        `).join('')}
                    </div>
                `;
            }
        }
        
        return `<div class="fallback-chart-placeholder">${data.length} data points available</div>`;
    }

    showNotification(message, type = 'info') {
        const notification = document.createElement('div');
        notification.className = `analytics-notification ${type}`;
        notification.innerHTML = `
            <div class="notification-content">
                <i class="fas fa-${this.getNotificationIcon(type)}"></i>
                <span>${message}</span>
                <button class="notification-close" onclick="this.parentElement.parentElement.remove()">×</button>
            </div>
        `;
        
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: rgba(15, 23, 42, 0.95);
            color: white;
            padding: 16px 20px;
            border-radius: 12px;
            border-left: 4px solid ${this.getNotificationColor(type)};
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
            z-index: 10000;
            animation: slideIn 0.3s ease;
            backdrop-filter: blur(10px);
            min-width: 300px;
            max-width: 400px;
        `;
        
        document.body.appendChild(notification);
        
        setTimeout(() => {
            notification.style.animation = 'slideOut 0.3s ease';
            setTimeout(() => notification.remove(), 300);
        }, 4000);
    }

    getNotificationIcon(type) {
        const icons = {
            success: 'check-circle',
            error: 'exclamation-circle',
            warning: 'exclamation-triangle',
            info: 'info-circle'
        };
        return icons[type] || 'info-circle';
    }

    getNotificationColor(type) {
        const colors = {
            success: '#10b981',
            error: '#ef4444',
            warning: '#f59e0b',
            info: '#3b82f6'
        };
        return colors[type] || '#3b82f6';
    }

    destroy() {
        console.log('🧹 Cleaning up analytics module...');
        
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
        }
        
        // Destroy Chart.js charts
        this.charts.forEach(chart => {
            if (chart && typeof chart.destroy === 'function') {
                chart.destroy();
            }
        });
        
        this.charts.clear();
        this.chartConfigs.clear();
        
        // Remove event listeners
        document.querySelectorAll('.analytics-control-btn').forEach(btn => {
            btn.removeEventListener('click', () => {});
        });
        
        console.log('✅ Analytics module cleaned up successfully');
    }
}

// Enhanced global export function for analytics data
function exportAnalyticsData(format) {
    console.log(`📤 Exporting premium analytics data in ${format} format`);
    
    const analyticsData = {
        timestamp: new Date().toISOString(),
        format: format,
        data: {
            performance_metrics: {
                cpu_usage: Math.floor(Math.random() * 30 + 10),
                memory_usage: Math.floor(Math.random() * 40 + 50),
                disk_usage: Math.floor(Math.random() * 30 + 40),
                network_throughput: Math.floor(Math.random() * 1000 + 500)
            },
            memory_distribution: {
                semantic: 28,
                episodic: 22,
                procedural: 18,
                working: 15,
                cached: 12,
                system: 5
            },
            agent_efficiency: {
                analytics: 92,
                memory: 88,
                graph: 85,
                query: 90,
                cache: 94,
                sync: 87
            },
            system_health: {
                uptime: 99.97,
                availability: 99.95,
                performance: 94.2,
                reliability: 99.1,
                security: 98.5,
                efficiency: 96.8
            }
        }
    };
    
    // Simulate export process
    const exportButton = document.querySelector(`button[onclick="exportAnalyticsData('${format}')"]`);
    if (exportButton) {
        const originalHTML = exportButton.innerHTML;
        exportButton.innerHTML = `<i class="fas fa-spinner fa-spin"></i><span>Exporting...</span>`;
        exportButton.disabled = true;
        
        setTimeout(() => {
            // Create and download file
            const dataStr = format === 'json' ? 
                JSON.stringify(analyticsData, null, 2) :
                convertToCSV(analyticsData);
            
            const dataBlob = new Blob([dataStr], { 
                type: format === 'json' ? 'application/json' : 'text/csv' 
            });
            
            const url = URL.createObjectURL(dataBlob);
            const link = document.createElement('a');
            link.href = url;
            link.download = `gaussos-analytics-${Date.now()}.${format}`;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            URL.revokeObjectURL(url);
            
            exportButton.innerHTML = originalHTML;
            exportButton.disabled = false;
            
            // Show success notification
            const notification = document.createElement('div');
            notification.textContent = `Analytics data exported successfully in ${format.toUpperCase()} format`;
            notification.style.cssText = `
                position: fixed;
                top: 20px;
                left: 50%;
                transform: translateX(-50%);
                background: linear-gradient(135deg, #10b981, #059669);
                color: white;
                padding: 16px 24px;
                border-radius: 12px;
                font-size: 14px;
                font-weight: 600;
                z-index: 10001;
                box-shadow: 0 8px 32px rgba(16, 185, 129, 0.4);
                animation: slideDown 0.3s ease;
            `;
            
            document.body.appendChild(notification);
            setTimeout(() => notification.remove(), 3000);
            
        }, 1500);
    }
}

function convertToCSV(data) {
    const rows = [];
    rows.push(['Metric', 'Value', 'Category', 'Timestamp']);
    
    Object.entries(data.data).forEach(([category, metrics]) => {
        if (typeof metrics === 'object') {
            Object.entries(metrics).forEach(([key, value]) => {
                rows.push([key, value, category, data.timestamp]);
            });
        }
    });
    
    return rows.map(row => row.join(',')).join('\n');
}

// Global exports
window.GaussOSAnalytics = GaussOSAnalytics;
window.exportAnalyticsData = exportAnalyticsData;

// Enhanced initialization with error handling
document.addEventListener('DOMContentLoaded', () => {
    const analyticsSection = document.getElementById('analytics');
    if (analyticsSection && analyticsSection.classList.contains('active')) {
        console.log('🚀 Analytics section is active, initializing premium charts...');
        try {
            new GaussOSAnalytics();
        } catch (error) {
            console.error('❌ Failed to initialize analytics:', error);
        }
    }
});

// Auto-initialize when section becomes active
const sectionObserver = new MutationObserver((mutations) => {
    mutations.forEach((mutation) => {
        if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
            const target = mutation.target;
            if (target.id === 'analytics' && target.classList.contains('active')) {
                if (!window.gaussOSAnalyticsInstance) {
                    console.log('🔄 Analytics section activated, initializing...');
                    try {
                        window.gaussOSAnalyticsInstance = new GaussOSAnalytics();
                    } catch (error) {
                        console.error('❌ Failed to initialize analytics on activation:', error);
                    }
                }
            }
        }
    });
});

// Observe the analytics section for class changes
const analyticsElement = document.getElementById('analytics');
if (analyticsElement) {
    sectionObserver.observe(analyticsElement, { 
        attributes: true, 
        attributeFilter: ['class'] 
    });
}

// Add CSS animations for enhanced UX
const style = document.createElement('style');
style.textContent = `
    @keyframes slideIn {
        from {
            opacity: 0;
            transform: translateX(100%);
        }
        to {
            opacity: 1;
            transform: translateX(0);
        }
    }
    
    @keyframes slideOut {
        from {
            opacity: 1;
            transform: translateX(0);
        }
        to {
            opacity: 0;
            transform: translateX(100%);
        }
    }
    
    @keyframes slideDown {
        from {
            opacity: 0;
            transform: translateX(-50%) translateY(-20px);
        }
        to {
            opacity: 1;
            transform: translateX(-50%) translateY(0);
        }
    }
    
    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }
    
    .loader-spinner {
        width: 40px;
        height: 40px;
        border: 3px solid rgba(255, 255, 255, 0.3);
        border-top: 3px solid #667eea;
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin-bottom: 12px;
    }
    
    .fallback-data-grid {
        display: grid;
        gap: 8px;
        margin-top: 16px;
        max-width: 300px;
    }
    
    .fallback-data-item {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px;
        background: rgba(255, 255, 255, 0.1);
        border-radius: 6px;
    }
    
    .color-indicator {
        width: 12px;
        height: 12px;
        border-radius: 50%;
        flex-shrink: 0;
    }
    
    .data-label {
        flex: 1;
        font-size: 12px;
    }
    
    .data-value {
        font-weight: 600;
        font-size: 13px;
    }
    
    .fallback-bar-chart {
        display: flex;
        flex-direction: column;
        gap: 12px;
        margin-top: 16px;
        width: 100%;
    }
    
    .fallback-bar-item {
        display: grid;
        grid-template-columns: 1fr 2fr auto;
        align-items: center;
        gap: 12px;
    }
    
    .bar-label {
        font-size: 11px;
        font-weight: 500;
    }
    
    .bar-container {
        height: 20px;
        background: rgba(255, 255, 255, 0.1);
        border-radius: 10px;
        overflow: hidden;
        position: relative;
    }
    
    .bar-fill {
        height: 100%;
        border-radius: 10px;
        transition: width 1s ease;
    }
    
    .bar-value {
        font-size: 12px;
        font-weight: 600;
        min-width: 40px;
        text-align: right;
    }
    
    .flow-controls {
        display: flex;
        gap: 8px;
    }
    
    .flow-toggle {
        padding: 6px 12px;
        background: rgba(255, 255, 255, 0.1);
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 6px;
        color: white;
        cursor: pointer;
        font-size: 12px;
        transition: all 0.2s ease;
    }
    
    .flow-toggle.active {
        background: #667eea;
        border-color: #667eea;
    }
    
    .flow-item {
        display: grid;
        grid-template-columns: 1fr 2fr;
        gap: 16px;
        align-items: center;
        padding: 16px;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 8px;
        transition: all 0.3s ease;
        animation: fadeIn 0.5s ease forwards;
        opacity: 0;
    }
    
    .flow-item:nth-child(1) { animation-delay: 0.1s; }
    .flow-item:nth-child(2) { animation-delay: 0.2s; }
    .flow-item:nth-child(3) { animation-delay: 0.3s; }
    .flow-item:nth-child(4) { animation-delay: 0.4s; }
    .flow-item:nth-child(5) { animation-delay: 0.5s; }
    .flow-item:nth-child(6) { animation-delay: 0.6s; }
    
    .flow-item:hover {
        background: rgba(255, 255, 255, 0.08);
        transform: translateX(4px);
    }
    
    .flow-operation {
        display: flex;
        align-items: center;
        gap: 12px;
    }
    
    .operation-icon {
        width: 32px;
        height: 32px;
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
        font-size: 14px;
    }
    
    .flow-bar-container {
        display: flex;
        align-items: center;
        gap: 12px;
    }
    
    .flow-bar {
        flex: 1;
        height: 12px;
        background: rgba(255, 255, 255, 0.2);
        border-radius: 6px;
        overflow: hidden;
    }
    
    .flow-fill {
        height: 100%;
        border-radius: 6px;
        transition: width 1.5s cubic-bezier(0.4, 0, 0.2, 1);
    }
    
    .flow-value {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 13px;
    }
    
    .flow-count {
        font-weight: 600;
    }
    
    .flow-percentage {
        opacity: 0.8;
        font-weight: 500;
    }
    
    .heatmap-legend {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 12px;
    }
    
    .legend-gradient {
        width: 60px;
        height: 8px;
        background: linear-gradient(90deg, rgba(102, 126, 234, 0.2), rgba(102, 126, 234, 1));
        border-radius: 4px;
    }
    
    .heatmap-data {
        transition: all 0.2s ease;
        cursor: pointer;
    }
    
    .heatmap-data:hover {
        transform: scale(1.1);
        z-index: 10;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    }
`;

document.head.appendChild(style);