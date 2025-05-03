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

    // Grab pages
    const pages = document.querySelectorAll('.content');
    
    pages.forEach(page => {

      // grab possible tabs
      const tabs = page.querySelectorAll('.tabs .tab');

      tabs.forEach(tab => {
        tab.addEventListener('click', () => {
          const data_target = tab.getAttribute('data-target');
          console.log('clicked on:' + data_target);

          tabs.forEach(tab => {
            tab.classList.remove('active');
          });

          tab.classList.add('active');

          // locate associated data
          const datas = page.querySelectorAll('.data');

          datas.forEach(data => {
            const data_id = data.id;
            console.log('data id: '+data_id);

            if (data_id == data_target) {
              data.style.display = 'block';
            } else {
              data.style.display = 'none';
            }
          });

        });
      });
    });
    "
        .to_string()
    }
}
