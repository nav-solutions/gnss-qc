use crate::prelude::QcReport;

impl QcReport {
    pub(crate) fn javascript(&self) -> String {
        "
    const sidebar = document.getElementById('sidebar');
    let inactivityTimer;

    function resetInactivityTimer() {
      clearTimeout(inactivityTimer);
      sidebar.classList.remove('hidden');
      inactivityTimer = setTimeout(() => {
        sidebar.classList.add('hidden');
      }, 5000);
    }

    // Réinitialise le timer sur interaction
    ['mousemove', 'keydown', 'click'].forEach(event => {
      document.addEventListener(event, resetInactivityTimer);
    });

    resetInactivityTimer(); // Démarrage initial

    // Navigation : afficher uniquement la section ciblée
    const navLinks = document.querySelectorAll('nav a');
    const sections = document.querySelectorAll('.section');

    navLinks.forEach(link => {
      link.addEventListener('click', () => {
        const targetId = link.getAttribute('data-target');
        sections.forEach(section => {
          section.classList.remove('active');
        });
        document.getElementById(targetId).classList.add('active');
      });
    });

    let currentConstellation = 'gps';

    const sourceTabs = document.querySelectorAll('#source-tabs .tab');
    const constellationTabs = document.querySelectorAll('#constellation-tabs .tab');
    
    // Gestion des clics sur les sources
    sourceTabs.forEach(tab => {
      tab.addEventListener('click', () => {
        currentSource = tab.getAttribute('data-source');
        updatePlot();
        sourceTabs.forEach(t => t.classList.remove('active'));
        tab.classList.add('active');
      });
    });

    // Gestion des clics sur les constellations
    constellationTabs.forEach(tab => {
      tab.addEventListener('click', () => {
        currentConstellation = tab.getAttribute('data-constellation');
        updatePlot();
        constellationTabs.forEach(t => t.classList.remove('active'));
        tab.classList.add('active');
      });
    });

    // Initialisation
    updatePlot();
    sourceTabs[0].classList.add('active');
    constellationTabs[0].classList.add('active');

    "
        .to_string()
    }
}
