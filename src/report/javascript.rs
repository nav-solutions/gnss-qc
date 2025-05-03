use crate::prelude::QcReport;

impl QcReport {
    pub(crate) fn javascript(&self) -> String {
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

    // Grab pages
    const pages = document.querySelectorAll('.content');
    
    pages.forEach(page => {

      // Data selection listener
      const data_selectors = page.querySelectorAll('.tabs .tab');

      data_selectors.forEach(selector => {

        // Event listener
        selector.addEventListener('click', () => {

          const target = selector.getAttribute('data-target');
          console.log('targetting :' + target);

          data_selectors.forEach(selector => {
            selector.classList.remove('active');
          });

          selector.classList.add('active');

          // Data selection or data filtering
          console.log('classes: '+ selector.classList);
          
          const data_set_selection = selector.getAttribute('filter') == null;

          if (data_set_selection) {
            // locate associated data
            const datas = page.querySelectorAll('.data');
  
            datas.forEach(data => {
              if (data.id == target) {
                data.style.display = 'block';
              } else {
                data.style.display = 'none';
              }
            });
          }

        });
      });
    });
    "
        .to_string()
    }
}
