use crate::report::QcRunReport;

impl QcRunReport {
    pub(crate) fn javascript() -> String {
        "
    lucide.createIcons();

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
    "
        .to_string()
    }
}
